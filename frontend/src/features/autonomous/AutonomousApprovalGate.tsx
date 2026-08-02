// ApprovalGate - displays an approval checkpoint and provides approve/reject.

import { CheckCircle2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { ApprovalRequest } from "@/types/autonomous";

interface ApprovalGateProps {
  request: ApprovalRequest;
  onApprove: () => void;
  onReject: () => void;
  busy: boolean;
}

export function ApprovalGate({ request, onApprove, onReject, busy }: ApprovalGateProps) {
  return (
    <div className="rounded-lg border-2 border-yellow-400/60 bg-yellow-400/10 p-4" data-testid="approval-gate">
      <div className="mb-3 flex items-center gap-2 text-sm">
        <div className="h-2 w-2 rounded-full bg-yellow-400 animate-pulse" />
        <span className="font-medium text-yellow-500">Approval Required</span>
      </div>
      <p className="mb-3 text-sm text-(--color-foreground)">{request.reason}</p>
      <p className="mb-3 text-xs font-mono text-(--color-muted-foreground)">
        Request ID: {request.request_id}
      </p>
      <div className="flex items-center justify-end gap-2">
        <Button
          variant="danger"
          size="sm"
          onClick={onReject}
          disabled={busy}
          data-testid="reject-button"
        >
          <XCircle className="h-4 w-4" />
          Reject
        </Button>
        <Button
          variant="primary"
          size="sm"
          onClick={onApprove}
          disabled={busy}
          data-testid="approve-button"
        >
          <CheckCircle2 className="h-4 w-4" />
          Approve
        </Button>
      </div>
    </div>
  );
}