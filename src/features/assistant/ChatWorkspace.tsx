import { AnimatePresence } from 'motion/react';
import * as motion from 'motion/react-m';
import { ArrowUp, ChevronRight, Cpu, ShieldCheck, Square, Radio, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import { time } from '@/lib/format';
import {
  MessageScrollerProvider,
  MessageScroller,
  MessageScrollerViewport,
  MessageScrollerContent,
  MessageScrollerItem,
  MessageScrollerButton,
} from '@/components/ui/message-scroller';
import {
  Message,
  MessageAvatar,
  MessageContent as ChatMessageContent,
  MessageHeader,
  MessageFooter,
} from '@/components/ui/message';
import { Bubble, BubbleContent } from '@/components/ui/bubble';
import {
  InputGroup,
  InputGroupTextarea,
  InputGroupAddon,
  InputGroupText,
  InputGroupButton,
} from '@/components/ui/input-group';
import { GenerationStatus } from './GenerationStatus';
import type { AssistantController } from './useAssistant';
import { lazy, Suspense } from 'react';
const MessageContent = lazy(() => import('./MessageContent'));
import { VoiceActivity, VoiceMicButton, VoiceSettingsButton } from '@/features/voice/VoiceControls';
import type { VoiceController } from '@/features/voice/useVoice';
import type { VoiceModeController } from '@/features/voice/useVoiceMode';
import { VoiceConversation, VoiceModeButton } from '@/features/voice/VoiceConversation';
const suggestions = [
  'Help me plan my day',
  'Explain a complex idea simply',
  'Help me think through a project',
];
export function ChatWorkspace({
  assistant,
  desktop,
  voice,
  voiceMode,
}: {
  assistant: AssistantController;
  desktop: boolean;
  voice: VoiceController;
  voiceMode: VoiceModeController;
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
    textarea,
    cancel,
    checkConnection,
  } = assistant;
  if (voiceMode.enabled) return <VoiceConversation mode={voiceMode} />;
  return (
    <MessageScrollerProvider key={selected ?? 'new'} autoScroll defaultScrollPosition="end">
      <div className="chat-heading">
        <div>
          <div className="eyebrow">INTELLIGENCE WORKSPACE</div>
          <h1>{selected ? title : 'Meet your next thought.'}</h1>
        </div>
        <div className="chat-mode-controls">
          <VoiceModeButton
            mode={voiceMode}
            disabled={
              !desktop ||
              busy ||
              loading ||
              voice.dictating ||
              connection !== 'online' ||
              !model ||
              !boot
            }
          />
          <Badge variant="outline" className="mode-badge">
            <ShieldCheck aria-hidden="true" size={14} /> Chat only
          </Badge>
        </div>
      </div>
      <MessageScroller className="chat-scroll">
        <MessageScrollerViewport aria-label="Conversation messages">
          <MessageScrollerContent
            className={messages.length ? 'messages' : 'chat-empty-content'}
            role="log"
            aria-label="Conversation"
            aria-busy={busy || loading}
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
                  {connection === 'online' && model
                    ? 'READY WHEN YOU ARE'
                    : 'YOUR LOCAL INTELLIGENCE'}
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
              <>
                {messages.map((message, index) => {
                  const next = messages[index + 1];
                  // Keep the user's scroll anchor stable when optimistic IDs become saved IDs.
                  const messageId =
                    message.role === 'user' && next?.role === 'assistant'
                      ? `user-${next.id}`
                      : message.id;
                  return (
                    <MessageScrollerItem
                      messageId={messageId}
                      scrollAnchor={message.role === 'user'}
                      key={messageId}
                    >
                      <Message
                        role="article"
                        align={message.role === 'user' ? 'end' : 'start'}
                        className={`chat-message ${message.role}`}
                      >
                        <MessageAvatar className="message-avatar">
                          {message.role === 'assistant' && message.status === 'streaming' ? (
                            <ActivityOrb state={thinking ? 'solving' : 'composing'} size={64} />
                          ) : (
                            <div className="avatar">
                              {message.role === 'assistant' ? (
                                <Cpu aria-hidden="true" size={17} />
                              ) : (
                                'K'
                              )}
                            </div>
                          )}
                        </MessageAvatar>
                        <ChatMessageContent className="message-body">
                          <MessageHeader className="message-meta">
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
                          </MessageHeader>
                          <Bubble
                            variant={message.role === 'user' ? 'muted' : 'ghost'}
                            align={message.role === 'user' ? 'end' : 'start'}
                            className={
                              message.role === 'assistant' ? 'assistant-bubble' : undefined
                            }
                          >
                            <BubbleContent className="message-content">
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
                            </BubbleContent>
                          </Bubble>
                          {message.role === 'assistant' &&
                            message.status === 'complete' &&
                            message.content.trim() && (
                              <MessageFooter className="message-actions">
                                <Button
                                  variant="ghost"
                                  size="icon"
                                  aria-label="Read reply aloud"
                                  title="Read reply aloud with the system voice"
                                  disabled={
                                    !voice.available || !!voice.pending || voice.stage !== 'idle'
                                  }
                                  onClick={() => void voice.speak(message)}
                                >
                                  <Volume2 className="size-4" aria-hidden="true" />
                                </Button>
                              </MessageFooter>
                            )}
                        </ChatMessageContent>
                      </Message>
                    </MessageScrollerItem>
                  );
                })}
              </>
            )}
          </MessageScrollerContent>
        </MessageScrollerViewport>
        <MessageScrollerButton aria-label="Latest message" className="chat-scroll-button" />
      </MessageScroller>
      <div className="composer-wrap">
        <VoiceActivity voice={voice} modeError={voiceMode.error} />
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
          className="composer-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (!voice.dictating) void send();
          }}
        >
          <InputGroup className="composer" aria-label="Message composer">
            <label className="sr-only" htmlFor="message">
              Message JARVIS
            </label>
            <InputGroupTextarea
              id="message"
              ref={textarea}
              value={draft}
              maxLength={16000}
              rows={2}
              placeholder="What’s on your mind?"
              aria-describedby="composer-guidance"
              onChange={(e) => setDraft(e.target.value)}
              disabled={busy || voice.dictating}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                  e.preventDefault();
                  if (!voice.dictating) void send();
                }
              }}
            />
            <InputGroupAddon align="block-end" className="composer-toolbar">
              <div className="composer-voice-tools">
                <VoiceMicButton voice={voice} disabled={busy || loading} />
                <VoiceSettingsButton voice={voice} />
              </div>
              <InputGroupText>
                <ShieldCheck aria-hidden="true" size={14} /> Stays on this device
              </InputGroupText>
              {busy ? (
                <InputGroupButton
                  variant="default"
                  size="icon-sm"
                  type="button"
                  className="send-button stop"
                  aria-label="Stop generation"
                  title="Stop generation"
                  onClick={() => void cancel()}
                >
                  <Square aria-hidden="true" className="size-4" fill="currentColor" />
                </InputGroupButton>
              ) : (
                <InputGroupButton
                  variant="default"
                  size="icon-sm"
                  className="send-button"
                  type="submit"
                  disabled={
                    !draft.trim() ||
                    !model ||
                    connection !== 'online' ||
                    !boot ||
                    loading ||
                    voice.dictating
                  }
                  aria-label="Send message"
                  title="Send message (Enter)"
                >
                  <ArrowUp aria-hidden="true" className="size-5" strokeWidth={2} />
                </InputGroupButton>
              )}
            </InputGroupAddon>
          </InputGroup>
        </form>
        <div className="composer-hint" id="composer-guidance">
          <span>Enter to send · Shift + Enter for a new line</span>
          <span>JARVIS can make mistakes. Check important details.</span>
        </div>
      </div>
    </MessageScrollerProvider>
  );
}
