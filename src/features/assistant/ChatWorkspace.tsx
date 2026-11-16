import { AnimatePresence } from 'motion/react';
import * as motion from 'motion/react-m';
import { ArrowDown, ArrowUp, ChevronRight, Cpu, ShieldCheck, Square, Radio } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import { Card } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import { time } from '@/lib/format';
import { GenerationStatus } from './GenerationStatus';
import type { AssistantController } from './useAssistant';
import { lazy, Suspense } from 'react';
const MessageContent = lazy(() => import('./MessageContent'));
const suggestions = [
  'Help me plan my day',
  'Explain a complex idea simply',
  'Help me think through a project',
];
export function ChatWorkspace({
  assistant,
  desktop,
}: {
  assistant: AssistantController;
  desktop: boolean;
}) {
  const {
    boot,
    busy,
    selected,
    title,
    loading,
    messages,
    thinking,
    draft,
    setDraft,
    send,
    model,
    connection,
    showLatest,
    setShowLatest,
    nearBottom,
    end,
    textarea,
    cancel,
    checkConnection,
  } = assistant;
  return (
    <>
      <div className="chat-heading">
        <div>
          <div className="eyebrow">INTELLIGENCE WORKSPACE</div>
          <h1>{selected ? title : 'Meet your next thought.'}</h1>
        </div>
        <Badge variant="outline" className="mode-badge">
          <ShieldCheck aria-hidden="true" size={14} /> Chat only
        </Badge>
      </div>
      <ScrollArea
        className="chat-scroll"
        viewportProps={{
          onScroll: (e) => {
            const el = e.currentTarget;
            nearBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 100;
            setShowLatest(!nearBottom.current);
          },
        }}
      >
        {loading ? (
          <div className="empty-state">
            <ActivityOrb state="connecting" />
            Loading conversation…
          </div>
        ) : messages.length === 0 ? (
          <motion.div
            className="welcome"
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.3 }}
          >
            <div className={`core ${busy ? 'working' : ''}`} aria-hidden="true">
              <div className="core-ring" />
              <div className="core-inner">
                <Cpu aria-hidden="true" size={38} strokeWidth={1} />
              </div>
              <span className="core-node" />
            </div>
            <div className="eyebrow">
              J.A.R.V.I.S. /{' '}
              {connection === 'online' && model ? 'READY WHEN YOU ARE' : 'YOUR LOCAL INTELLIGENCE'}
            </div>
            <h2>
              A little clarity.
              <br />
              <span>A lot of possibility.</span>
            </h2>
            <p>
              A private space to think, plan, and build.
              <br />
              Powered by your local model. Guided by you.
            </p>
            <div className="suggestions">
              {suggestions.map((s) => (
                <Button
                  variant="ghost"
                  key={s}
                  onClick={() => {
                    setDraft(s);
                    textarea.current?.focus();
                  }}
                >
                  {s}
                  <ChevronRight aria-hidden="true" size={16} />
                </Button>
              ))}
            </div>
          </motion.div>
        ) : (
          <div className="messages" role="log" aria-label="Conversation" aria-busy={busy}>
            {messages.map((message) => (
              <Card role="article" className={`message ${message.role}`} key={message.id}>
                <div className="message-avatar">
                  {message.role === 'assistant' && message.status === 'streaming' ? (
                    <ActivityOrb state={thinking ? 'solving' : 'composing'} size={64} />
                  ) : (
                    <div className="avatar">
                      {message.role === 'assistant' ? <Cpu aria-hidden="true" size={17} /> : 'K'}
                    </div>
                  )}
                </div>
                <div className="message-body">
                  <div className="message-meta">
                    <strong>{message.role === 'assistant' ? 'JARVIS' : 'You'}</strong>
                    <span>{time(message.created_at)}</span>
                    {message.status !== 'complete' && (
                      <span className="message-status">
                        {message.status === 'streaming'
                          ? thinking
                            ? 'Thinking…'
                            : 'Generating…'
                          : message.status}
                      </span>
                    )}
                  </div>
                  <div className="message-content">
                    {message.content && message.role === 'assistant' ? (
                      <Suspense fallback={<span>{message.content}</span>}>
                        <MessageContent content={message.content} />
                      </Suspense>
                    ) : (
                      message.content ||
                      (message.status === 'streaming'
                        ? 'Preparing a response…'
                        : 'No response saved.')
                    )}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        )}
        <div ref={end} />
      </ScrollArea>
      <div className="composer-wrap">
        {showLatest && (
          <Button
            variant="ghost"
            className="latest-button"
            onClick={() => {
              nearBottom.current = true;
              setShowLatest(false);
              end.current?.scrollIntoView({ behavior: 'instant' });
            }}
          >
            <ArrowDown size={14} aria-hidden="true" />
            Latest message
          </Button>
        )}
        <AnimatePresence initial={false}>
          {busy && <GenerationStatus key="generation" thinking={thinking} />}
        </AnimatePresence>
        {desktop && !busy && connection !== 'checking' && (!model || connection === 'offline') && (
          <div className="composer-connection-hint" id="connection-guidance">
            <Radio aria-hidden="true" size={16} />
            <span>
              {connection === 'offline'
                ? 'Start the local server in LM Studio to begin.'
                : 'Load a chat model in LM Studio to begin.'}
            </span>
            <Button variant="ghost" onClick={() => void checkConnection()}>
              Check again
            </Button>
          </div>
        )}
        <form
          className="composer"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <label className="sr-only" htmlFor="message">
            Message JARVIS
          </label>
          <Textarea
            id="message"
            ref={textarea}
            value={draft}
            maxLength={16000}
            rows={2}
            placeholder="What’s on your mind?"
            aria-describedby="composer-guidance"
            onChange={(e) => setDraft(e.target.value)}
            disabled={busy}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault();
                void send();
              }
            }}
          />
          <div className="composer-toolbar">
            <span>
              <ShieldCheck aria-hidden="true" size={14} /> Stays on this device
            </span>
            {busy ? (
              <Button
                variant="default"
                size="icon"
                type="button"
                className="send-button stop"
                aria-label="Stop generation"
                title="Stop generation"
                onClick={() => void cancel()}
              >
                <Square aria-hidden="true" className="size-4" fill="currentColor" />
              </Button>
            ) : (
              <Button
                variant="default"
                size="icon"
                className="send-button"
                type="submit"
                disabled={!draft.trim() || !model || connection !== 'online' || !boot || loading}
                aria-label="Send message"
                title="Send message (Enter)"
              >
                <ArrowUp aria-hidden="true" className="size-5" strokeWidth={2} />
              </Button>
            )}
          </div>
        </form>
        <div className="composer-hint" id="composer-guidance">
          <span>Enter to send · Shift + Enter for a new line</span>
          <span>JARVIS can make mistakes. Check important details.</span>
        </div>
      </div>
    </>
  );
}
