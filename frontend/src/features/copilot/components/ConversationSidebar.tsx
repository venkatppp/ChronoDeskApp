// ConversationSidebar - Recent conversations with search and management

import { useState, useMemo } from "react";
import { Search, MessageSquarePlus, Trash2, MoreVertical } from "lucide-react";
import { cn } from "@/utils/cn";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import type { Conversation } from "@/types/copilot";

interface ConversationSidebarProps {
  conversations: Conversation[];
  currentConversationId: string | null;
  onSelect: (conversationId: string) => void;
  onNew: () => void;
  onDelete: (conversationId: string) => void;
}

export function ConversationSidebar({
  conversations,
  currentConversationId,
  onSelect,
  onNew,
  onDelete,
}: ConversationSidebarProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [activeMenu, setActiveMenu] = useState<string | null>(null);

  const filteredConversations = useMemo(() => {
    if (!searchQuery) return conversations;
    const query = searchQuery.toLowerCase();
    return conversations.filter((conv) => conv.title.toLowerCase().includes(query));
  }, [conversations, searchQuery]);

  const handleDelete = (conversationId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (confirm("Delete this conversation?")) {
      onDelete(conversationId);
      setActiveMenu(null);
    }
  };

  return (
    <div className="flex h-full w-80 shrink-0 flex-col border-r border-(--color-border) bg-(--color-surface)">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-(--color-border) p-4">
        <h2 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
          Copilot
        </h2>
        <button
          onClick={onNew}
          className="flex h-8 w-8 items-center justify-center rounded-lg bg-(--color-accent) text-(--color-accent-foreground) transition-colors hover:bg-(--color-accent)/90"
          title="New conversation"
        >
          <MessageSquarePlus className="h-4 w-4" />
        </button>
      </div>

      {/* Search */}
      <div className="p-3">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-muted-foreground)" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search conversations..."
            className="w-full rounded-lg border border-(--color-border) bg-(--color-surface-raised) py-2 pl-9 pr-3 text-sm text-(--color-foreground) placeholder:text-(--color-muted-foreground) focus:border-(--color-accent) focus:outline-none focus:ring-1 focus:ring-(--color-accent)"
          />
        </div>
      </div>

      {/* Conversations List */}
      <div className="flex-1 overflow-y-auto px-2">
        {filteredConversations.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-4 text-center">
            <MessageSquarePlus className="h-8 w-8 text-(--color-muted-foreground)" />
            <p className="text-sm text-(--color-muted-foreground)">
              {searchQuery ? "No conversations found" : "Start a new conversation"}
            </p>
          </div>
        ) : (
          <div className="space-y-1 py-2">
            {filteredConversations.map((conversation) => {
              const isActive = conversation.id === currentConversationId;
              return (
                <div
                  key={conversation.id}
                  onClick={() => onSelect(conversation.id)}
                  className={cn(
                    "group relative flex cursor-pointer flex-col gap-1 rounded-lg p-3 transition-colors",
                    isActive
                      ? "bg-(--color-accent-muted) text-(--color-accent)"
                      : "hover:bg-(--color-surface-hover) text-(--color-foreground)"
                  )}
                >
                  <div className="flex items-start justify-between gap-2">
                    <span className="flex-1 truncate text-sm font-medium">{conversation.title}</span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setActiveMenu(activeMenu === conversation.id ? null : conversation.id);
                      }}
                      className="opacity-0 transition-opacity group-hover:opacity-100"
                    >
                      <MoreVertical className="h-4 w-4" />
                    </button>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-(--color-muted-foreground)">
                    <span>{formatRelativeTime(conversation.updated_at)}</span>
                    <span>·</span>
                    <span>{conversation.message_count} messages</span>
                  </div>

                  {/* Context Menu */}
                  {activeMenu === conversation.id && (
                    <div className="absolute right-2 top-12 z-10 w-48 rounded-lg border border-(--color-border) bg-(--color-surface-raised) py-1 shadow-lg">
                      <button
                        onClick={(e) => handleDelete(conversation.id, e)}
                        className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-(--color-danger) hover:bg-(--color-surface-hover)"
                      >
                        <Trash2 className="h-4 w-4" />
                        <span>Delete</span>
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
