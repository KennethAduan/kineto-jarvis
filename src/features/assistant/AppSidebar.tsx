import {
  Activity as ActivityIcon,
  Cpu,
  Plus,
  MessageSquare,
  ShieldCheck,
  Trash2,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Sidebar,
  SidebarHeader,
  SidebarContent,
  SidebarFooter,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from '@/components/ui/sidebar';
import type { AssistantController } from './useAssistant';
export function AppSidebar({ assistant }: { assistant: AssistantController }) {
  const { boot, busy, page, setPage, setDeleteId, conversations, selected, openConversation } =
    assistant;
  return (
    <Sidebar collapsible="none" className="sidebar" aria-label="Workspace" role="complementary">
      <SidebarHeader className="p-0 gap-0">
        <div className="brand">
          <div className="brand-mark">
            <Cpu aria-hidden="true" size={22} />
          </div>
          <div>
            KINETO<span>LOCAL ASSISTANT</span>
          </div>
        </div>
        <Button
          variant="ghost"
          className="new-chat"
          disabled={busy || !boot}
          onClick={() => void openConversation(null)}
        >
          <Plus aria-hidden="true" size={18} /> New conversation{' '}
          <span className="shortcut">＋</span>
        </Button>
      </SidebarHeader>
      <SidebarContent className="gap-0 overflow-hidden">
        <div className="section-label">WORKSPACE</div>
        <nav aria-label="Main navigation">
          <SidebarMenu>
            {(
              [
                ['chat', MessageSquare, 'Assistant'],
                ['activity', ActivityIcon, 'Task activity'],
                ['permissions', ShieldCheck, 'Permissions'],
              ] as const
            ).map(([id, Icon, label]) => (
              <SidebarMenuItem key={id}>
                <SidebarMenuButton
                  isActive={page === id}
                  className={`nav-item ${page === id ? 'active' : ''}`}
                  aria-current={page === id ? 'page' : undefined}
                  onClick={() => {
                    setPage(id);
                    setDeleteId(null);
                  }}
                >
                  <Icon aria-hidden="true" size={18} />
                  {label}
                  {page === id && <span className="active-dot" />}
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </nav>
        <div className="section-label recent-label">
          CONVERSATIONS <span>{conversations.length.toString().padStart(2, '0')}</span>
        </div>
        <div className="conversation-list">
          {conversations.length === 0 ? (
            <p className="quiet">Your conversations stay on this device.</p>
          ) : (
            conversations.map((c) => (
              <div className={`conversation-row ${selected === c.id ? 'selected' : ''}`} key={c.id}>
                <Button
                  variant="ghost"
                  className="conversation-title"
                  disabled={busy}
                  onClick={() => void openConversation(c.id)}
                  title={c.title}
                >
                  <MessageSquare aria-hidden="true" size={14} />
                  <span>{c.title}</span>
                </Button>
                <Button
                  variant="ghost"
                  className="icon-button delete"
                  disabled={busy}
                  aria-label={`Delete ${c.title}`}
                  onClick={() => setDeleteId(c.id)}
                >
                  <Trash2 aria-hidden="true" size={14} />
                </Button>
              </div>
            ))
          )}
        </div>
      </SidebarContent>
      <SidebarFooter className="p-0">
        <div className="sidebar-bottom">
          <ShieldCheck aria-hidden="true" size={17} />
          <div>
            Local by design<span>No cloud services connected</span>
          </div>
        </div>
      </SidebarFooter>
    </Sidebar>
  );
}
