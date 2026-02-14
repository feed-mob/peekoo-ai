import { PanelShell } from "@/components/panels/PanelShell";
import { ChatPanel } from "@/features/chat/ChatPanel";

export default function ChatView() {
  return (
    <PanelShell title="Chat">
      <ChatPanel />
    </PanelShell>
  );
}
