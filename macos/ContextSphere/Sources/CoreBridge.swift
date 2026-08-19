import Foundation

// MARK: - RPC protocol

/// Incoming message from the daemon (stdout is a JSON-RPC stream).
enum RpcIncoming: Decodable {
    case response(id: Int, result: JSONValue?, error: RpcError?)
    case notification(event: String, payload: JSONValue?)

    private enum CodingKeys: String, CodingKey { case id, result, error, event, payload }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        if c.contains(.event) {
            self = .notification(event: try c.decode(String.self, forKey: .event),
                                 payload: try c.decodeIfPresent(JSONValue.self, forKey: .payload))
        } else {
            self = .response(id: try c.decode(Int.self, forKey: .id),
                             result: try c.decodeIfPresent(JSONValue.self, forKey: .result),
                             error: try c.decodeIfPresent(RpcError.self, forKey: .error))
        }
    }
}

struct RpcError: Decodable {
    let message: String
}

/// Minimal JSON value tree so raw payloads can be passed through to typed
/// decoders without forcing every request to pre-declare its result type.
enum JSONValue: Decodable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .null; return }
        if let b = try? c.decode(Bool.self) { self = .bool(b); return }
        if let n = try? c.decode(Double.self) { self = .number(n); return }
        if let s = try? c.decode(String.self) { self = .string(s); return }
        if let a = try? c.decode([JSONValue].self) { self = .array(a); return }
        if let o = try? c.decode([String: JSONValue].self) { self = .object(o); return }
        throw DecodingError.dataCorruptedError(in: c, debugDescription: "unknown JSON value")
    }

    func toData() throws -> Data {
        switch self {
        case .null: return Data("null".utf8)
        case .bool(let b): return Data(b ? "true".utf8 : "false".utf8)
        case .number(let n): return Data(String(format: "%.17g", n).utf8)
        case .string(let s): return try JSONSerialization.data(withJSONObject: s)
        case .array(let a): return try JSONSerialization.data(withJSONObject: a.map(\.jsonObject))
        case .object(let o): return try JSONSerialization.data(withJSONObject: o.mapValues(\.jsonObject))
        }
    }

    var jsonObject: Any {
        switch self {
        case .null: return NSNull()
        case .bool(let b): return b
        case .number(let n): return n
        case .string(let s): return s
        case .array(let a): return a.map(\.jsonObject)
        case .object(let o): return o.mapValues(\.jsonObject)
        }
    }
}

extension JSONValue: Equatable, Hashable {
    /// The object member of a JSON value, or `nil` when not an object.
    var objectValue: [String: JSONValue]? {
        guard case .object(let object) = self else { return nil }
        return object
    }

    static func == (lhs: JSONValue, rhs: JSONValue) -> Bool {
        switch (lhs, rhs) {
        case (.null, .null): true
        case (.bool(let a), .bool(let b)): a == b
        case (.number(let a), .number(let b)): a == b
        case (.string(let a), .string(let b)): a == b
        case (.array(let a), .array(let b)): a == b
        case (.object(let a), .object(let b)): a == b
        default: false
        }
    }

    func hash(into hasher: inout Hasher) {
        switch self {
        case .null: hasher.combine(0)
        case .bool(let b): hasher.combine(1); hasher.combine(b)
        case .number(let n): hasher.combine(2); hasher.combine(n)
        case .string(let s): hasher.combine(3); hasher.combine(s)
        case .array(let a): hasher.combine(4); hasher.combine(a)
        case .object(let o): hasher.combine(5); hasher.combine(o)
        }
    }
}

// MARK: - Core bridge

/// Spawns the `contextsphere_core` daemon and speaks line-delimited JSON-RPC
/// over its stdin/stdout. Responses are matched to pending requests by id;
/// daemon events arrive as notifications.
@MainActor
final class CoreBridge: ObservableObject {
    static let shared = CoreBridge()

    @Published var isRunning = false
    @Published var backendVersion: String?
    @Published var lastError: String?

    var onEvent: ((String, Data?) -> Void)?

    private var process: Process?
    private var stdinPipe: Pipe?
    private var stdoutPipe: Pipe?
    private var nextId = 1
    private var pending: [Int: CheckedContinuation<Data?, Error>] = [:]
    private var buffer = Data()

    private init() {}

    func start() {
        guard process == nil else { return }
        let daemonURL = resolveDaemonURL()
        guard FileManager.default.isExecutableFile(atPath: daemonURL.path) else {
            lastError = "core daemon not found at \(daemonURL.path)"
            return
        }

        let p = Process()
        p.executableURL = daemonURL
        p.standardInput = Pipe()
        p.standardOutput = Pipe()
        p.standardError = FileHandle.standardError
        p.terminationHandler = { [weak self] _ in
            Task { @MainActor in
                self?.process = nil
                self?.isRunning = false
                self?.failAllPending("daemon exited")
            }
        }

        stdinPipe = p.standardInput as? Pipe
        stdoutPipe = p.standardOutput as? Pipe
        stdoutPipe?.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            guard !data.isEmpty else { return }
            Task { @MainActor in self?.consume(data) }
        }

        do {
            try p.run()
            process = p
            isRunning = true
        } catch {
            lastError = "failed to launch daemon: \(error.localizedDescription)"
        }
    }

    func stop() {
        process?.terminate()
    }

    /// Finds the daemon: bundled `Contents/MacOS/contextsphere_core`, else the
    /// `CONTEXTSPHERE_CORE` env override (dev builds use the cargo target dir).
    private func resolveDaemonURL() -> URL {
        if let env = ProcessInfo.processInfo.environment["CONTEXTSPHERE_CORE"] {
            return URL(fileURLWithPath: env)
        }
        let bundle = Bundle.main
        if let bundled = bundle.executableURL?.deletingLastPathComponent()
            .appendingPathComponent("contextsphere_core"),
            FileManager.default.isExecutableFile(atPath: bundled.path) {
            return bundled
        }
        return URL(fileURLWithPath: "src-tauri/target/debug/contextsphere_core")
    }

    /// Sends a request and decodes the result into `T`. Errors (RPC-level
    /// or decode-level) throw `CoreError`.
    func request<T: Decodable>(_ method: String, params: [String: Any] = [:],
                               as type: T.Type) async throws -> T {
        let data = try await rawRequest(method, params: params)
        guard let data else { throw CoreError.emptyResult }
        do {
            return try JSONDecoder().decode(T.self, from: data)
        } catch {
            throw CoreError.decode("\(method): \(error.localizedDescription)")
        }
    }

    /// Convenience for commands that return no meaningful payload. The
    /// daemon serializes `Ok(())` as a JSON `null` result, which counts
    /// as success here (typed requests still reject it).
    func call(_ method: String, params: [String: Any] = [:]) async throws {
        _ = try await rawRequest(method, params: params)
    }

    private func rawRequest(_ method: String, params: [String: Any]) async throws -> Data? {
        let id = nextId
        nextId += 1
        let payload = try JSONSerialization.data(withJSONObject: [
            "id": id, "method": method, "params": params,
        ])
        return try await withCheckedThrowingContinuation { (cont: CheckedContinuation<Data?, Error>) in
            pending[id] = cont
            guard let stdin = stdinPipe?.fileHandleForWriting else {
                pending.removeValue(forKey: id)
                cont.resume(throwing: CoreError.notRunning)
                return
            }
            var line = payload
            line.append(0x0A)
            do {
                try stdin.write(contentsOf: line)
            } catch {
                pending.removeValue(forKey: id)
                cont.resume(throwing: CoreError.transport(error.localizedDescription))
            }
        }
    }

    private func consume(_ data: Data) {
        buffer.append(data)
        while let newline = buffer.firstIndex(of: 0x0A) {
            let line = buffer.subdata(in: buffer.startIndex..<newline)
            buffer.removeSubrange(buffer.startIndex...newline)
            guard let obj = try? JSONDecoder().decode(RpcIncoming.self, from: line) else { continue }
            switch obj {
            case .response(let id, let result, let error):
                if let cont = pending.removeValue(forKey: id) {
                    if let error {
                        cont.resume(throwing: CoreError.rpc(error.message))
                    } else {
                        cont.resume(returning: try? result?.toData())
                    }
                }
            case .notification(let event, let payload):
                onEvent?(event, try? payload?.toData())
            }
        }
    }

    private func failAllPending(_ reason: String) {
        for (_, cont) in pending {
            cont.resume(throwing: CoreError.transport(reason))
        }
        pending.removeAll()
    }
}

enum CoreError: LocalizedError {
    case notRunning
    case transport(String)
    case rpc(String)
    case emptyResult
    case decode(String)

    var errorDescription: String? {
        switch self {
        case .notRunning: return "core daemon is not running"
        case .transport(let s): return s
        case .rpc(let s): return s
        case .emptyResult: return "empty result"
        case .decode(let s): return s
        }
    }
}