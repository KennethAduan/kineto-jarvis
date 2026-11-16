import {
  Activity as ActivityIcon,
  LoaderCircle,
  Check,
  CircleAlert,
  ShieldCheck,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import * as motion from 'motion/react-m';
import { time } from '@/lib/format';
import type { Activity, Permission } from '@/domain/models';

export function ActivityView({ activity }: { activity: Activity[] }) {
  return (
    <motion.section
      className="detail-page"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
    >
      <div className="eyebrow">OBSERVABILITY</div>
      <h1>Task activity</h1>
      <p>
        Real generation activity, saved locally. Partial responses are retained when you stop a
        task.
      </p>
      {activity.length === 0 ? (
        <div className="empty-card">
          <ActivityIcon aria-hidden="true" size={28} />
          <h2>A clear runway.</h2>
          <p>Your first chat will appear here.</p>
        </div>
      ) : (
        <div className="activity-list">
          {activity.map((item) => (
            <article key={item.id}>
              <div className="activity-icon">
                {item.status === 'running' ? (
                  <LoaderCircle aria-hidden="true" className="spin" size={18} />
                ) : item.status === 'complete' ? (
                  <Check aria-hidden="true" size={18} />
                ) : (
                  <CircleAlert aria-hidden="true" size={18} />
                )}
              </div>
              <div>
                <strong>Local response</strong>
                <p>{item.detail}</p>
                <small>
                  {item.model} · {time(item.created_at)}
                </small>
              </div>
              <Badge variant="outline" className={`status-badge ${item.status}`}>
                {item.status}
              </Badge>
            </article>
          ))}
        </div>
      )}
    </motion.section>
  );
}

export function PermissionsView({ permissions }: { permissions: Permission[] }) {
  return (
    <motion.section
      className="detail-page"
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
    >
      <div className="eyebrow">YOU ARE IN CONTROL</div>
      <h1>Permission boundaries</h1>
      <p>
        Capabilities are enforced in the native app. Future tools will need scoped approval before
        they can act.
      </p>
      <div className="permission-list">
        {permissions.map((item) => (
          <article key={item.capability}>
            <ShieldCheck aria-hidden="true" size={22} />
            <div>
              <strong>{item.capability}</strong>
              <p>{item.description}</p>
            </div>
            <Badge variant="outline" className={`status-badge ${item.state}`}>
              {item.state}
            </Badge>
          </article>
        ))}
      </div>
      <div className="privacy-note">
        Conversation history is stored as an unencrypted SQLite database in the app’s data folder.
        Anyone with access to your operating system account may be able to read it. Deleting a
        conversation removes it from the app; it is not a forensic secure erase.
      </div>
    </motion.section>
  );
}
