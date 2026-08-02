import { memo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { Copy } from "lucide-react";

interface MarkdownRendererProps {
  content: string;
}

function MarkdownRendererComponent({ content }: MarkdownRendererProps) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      rehypePlugins={[rehypeHighlight]}
      components={{
        a: ({ children, href, ...props }) => (
          <a
            href={href}
            rel="noreferrer noopener"
            target="_blank"
            className="text-(--color-accent) underline underline-offset-2 hover:text-(--color-accent)/80"
            {...props}
          >
            {children}
          </a>
        ),
        code: ({ inline, className, children, ...props }: any) => {
          const match = /language-(\w+)/.exec(className || "");
          return !inline ? (
            <div className="relative my-2 overflow-hidden rounded-lg border border-(--color-border) bg-(--color-background)">
              <div className="flex items-center justify-between border-b border-(--color-border) bg-(--color-surface-raised) px-3 py-1.5">
                <span className="text-xs font-mono text-(--color-muted-foreground)">
                  {match?.[1] || "code"}
                </span>
                <button
                  type="button"
                  onClick={() => navigator.clipboard.writeText(String(children))}
                  className="rounded p-1 text-xs text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                  aria-label="Copy code block"
                >
                  <Copy className="h-3.5 w-3.5" />
                </button>
              </div>
              <pre className="m-0 overflow-x-auto p-3">
                <code className={className} {...props}>
                  {children}
                </code>
              </pre>
            </div>
          ) : (
            <code
              className="rounded bg-(--color-surface-raised) px-1.5 py-0.5 font-mono text-xs text-(--color-accent)"
              {...props}
            >
              {children}
            </code>
          );
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

export const MarkdownRenderer = memo(MarkdownRendererComponent);
