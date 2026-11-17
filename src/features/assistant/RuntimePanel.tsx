import { Radio, RefreshCw, MessageSquare, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { time } from '@/lib/format';
import type { AssistantController } from './useAssistant';
export function RuntimePanel({
  assistant,
  desktop,
}: {
  assistant: AssistantController;
  desktop: boolean;
}) {
  const { connection, busy, checkConnection, model, setModel, models, connectionError, checkedAt } =
    assistant;
  return (
    <aside className="inspector" aria-label="Local runtime">
      <div className="inspector-title">
        <Radio aria-hidden="true" size={16} /> LOCAL RUNTIME
      </div>
      <div className="runtime-card">
        <div className="card-label">
          INFERENCE SERVER
          <Button
            variant="ghost"
            className="icon-button"
            aria-label="Refresh models and connection"
            disabled={connection === 'checking' || busy || !desktop}
            onClick={() => void checkConnection()}
          >
            <RefreshCw
              aria-hidden="true"
              size={15}
              className={connection === 'checking' ? 'spin' : ''}
            />
          </Button>
        </div>
        <strong>LM Studio</strong>
        <code>127.0.0.1:1234/v1</code>
        <div className={`connection-state ${connection}`}>
          <span className={`connection-dot ${connection}`} />
          {connection === 'online'
            ? 'Connected'
            : connection === 'checking'
              ? 'Checking…'
              : 'Offline'}
        </div>
      </div>
      <label className="section-label" htmlFor="model">
        ACTIVE MODEL
      </label>
      <select
        id="model"
        value={model}
        onChange={(e) => setModel(e.target.value)}
        disabled={busy || !models.length}
      >
        {!models.length && <option value="">No model discovered</option>}
        {models.map((m) => (
          <option key={m.id} value={m.id}>
            {m.id}
          </option>
        ))}
      </select>
      <div className="model-note">
        Preferred: qwen/qwen3-4b-2507
        <br />
        Uses the exact model ID from LM Studio.
      </div>
      {checkedAt && <div className="checked-at">Last checked {time(checkedAt)}</div>}
      {connectionError && (
        <div className="connection-help" role="status">
          {connectionError}
        </div>
      )}
      <div className="runtime-divider" />
      <div className="section-label">CAPABILITY SCOPE</div>
      <div className="scope-row">
        <MessageSquare aria-hidden="true" size={16} />
        <span>Conversation</span>
        <span className="scope-value">Enabled</span>
      </div>
      <div className="scope-row">
        <ShieldCheck aria-hidden="true" size={16} />
        <span>Computer control</span>
        <span className="muted">Off</span>
      </div>
      <div className="inspector-note">
        <div className="small-orbit" aria-hidden="true" />
        <strong>
          Your machine.
          <br />
          Your intelligence.
        </strong>
        <p>
          No cloud fallback.
          <br />
          No background tool execution.
        </p>
      </div>
      <div className="runtime-version">KINETO / FOUNDATION 001</div>
    </aside>
  );
}
