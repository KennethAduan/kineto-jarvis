import { ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { SidebarProvider } from '@/components/ui/sidebar';
import { api, desktop } from './infrastructure/tauri/assistant-api';
import { useAssistant } from './features/assistant/useAssistant';
import { AppSidebar } from './features/assistant/AppSidebar';
import { ChatWorkspace } from './features/assistant/ChatWorkspace';
import { RuntimePanel } from './features/assistant/RuntimePanel';
import { ActivityView, PermissionsView } from './features/assistant/WorkspacePages';
import { DeleteConversationDialog } from './features/assistant/DeleteConversationDialog';
import { voiceApi } from './infrastructure/tauri/voice-api';
import { useVoice } from './features/voice/useVoice';
import { VoiceSetup } from './features/voice/VoiceControls';
export default function App() {
  const assistant = useAssistant(api, desktop);
  const voice = useVoice(voiceApi, desktop, assistant.selected, (text) => {
    const combined = assistant.draft ? `${assistant.draft}\n${text}` : text;
    if (combined.length > 16_000) {
      assistant.setError('The draft is too long to add dictation. Shorten it, then record again.');
      return;
    }
    assistant.setDraft(combined);
  });
  const {
    boot,
    page,
    error,
    setError,
    deleteId,
    setDeleteId,
    removeConversation,
    activity,
    connection,
    busy,
  } = assistant;

  return (
    <SidebarProvider className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to workspace
      </a>
      <AppSidebar assistant={assistant} />
      <VoiceSetup voice={voice} />

      <main id="main-content" tabIndex={-1}>
        <header className="topbar">
          <div className="breadcrumb">
            Workspace <ChevronRight aria-hidden="true" size={14} />
            <strong>
              {page === 'chat'
                ? 'Assistant'
                : page === 'activity'
                  ? 'Task activity'
                  : 'Permissions'}
            </strong>
          </div>
          <span className="platform-label">
            {boot?.platform.name ?? (desktop ? 'Starting desktop' : 'Browser preview')}{' '}
            <span className="tiny-dot" /> MVP 0.1
          </span>
        </header>
        {!desktop && (
          <div className="notice">
            Browser preview · Open the desktop app with <code>pnpm desktop</code> to connect to LM
            Studio and save conversations.
          </div>
        )}
        {error && (
          <div className="error-banner" role="alert">
            <span>{error}</span>
            <Button variant="ghost" onClick={() => setError('')} aria-label="Dismiss error">
              Dismiss
            </Button>
          </div>
        )}
        <DeleteConversationDialog
          open={deleteId !== null}
          onClose={() => setDeleteId(null)}
          onConfirm={() => (deleteId ? removeConversation(deleteId) : Promise.resolve())}
        />

        {page === 'chat' ? (
          <ChatWorkspace assistant={assistant} desktop={desktop} voice={voice} />
        ) : page === 'activity' ? (
          <ActivityView activity={activity} />
        ) : (
          <PermissionsView permissions={boot?.permissions ?? []} />
        )}
        <footer className="statusbar" role="status" aria-live="polite">
          <span>
            <span className={`connection-dot ${connection}`} />
            {connection === 'online'
              ? 'LM Studio connected'
              : connection === 'checking'
                ? 'Checking connection'
                : 'LM Studio offline'}
          </span>
          <span>{busy ? 'Generating locally' : 'All tools disabled'}</span>
        </footer>
      </main>

      <RuntimePanel assistant={assistant} desktop={desktop} />
    </SidebarProvider>
  );
}
