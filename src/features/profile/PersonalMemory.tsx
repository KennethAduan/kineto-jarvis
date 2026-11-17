import { useEffect, useState } from 'react';
import { Brain, ShieldCheck, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet';
import type { PersonalProfile, ProfileGateway } from '@/domain/profile';
import { errorText } from '@/lib/errors';

const empty: PersonalProfile = { display_name: '', content: '', enabled: false };
export function PersonalMemory({
  api,
  open,
  onOpenChange,
  disabled,
}: {
  api: ProfileGateway;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  disabled: boolean;
}) {
  const [profile, setProfile] = useState(empty);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [saved, setSaved] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [forget, setForget] = useState(false);
  useEffect(() => {
    if (!open) return;
    let current = true;
    setPending(true);
    setLoaded(false);
    setError('');
    setSaved(false);
    setForget(false);
    void api
      .get()
      .then((data) => {
        if (current) {
          setProfile(data);
          setLoaded(true);
        }
      })
      .catch((error) => {
        if (current) setError(errorText(error));
      })
      .finally(() => {
        if (current) setPending(false);
      });
    return () => {
      current = false;
    };
  }, [api, open]);
  async function save(next: PersonalProfile) {
    setPending(true);
    setError('');
    setSaved(false);
    try {
      await api.save(next);
      setProfile(next);
      setSaved(true);
      setForget(false);
    } catch (error) {
      setError(errorText(error));
    } finally {
      setPending(false);
    }
  }
  const locked = pending || disabled || !loaded;
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="personal-memory">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <Brain size={19} /> Personal memory
          </SheetTitle>
          <SheetDescription>What JARVIS knows about you, stored on this device.</SheetDescription>
        </SheetHeader>
        <div className="memory-body">
          <div className="memory-privacy">
            <ShieldCheck size={18} />
            <span>Local profile · Your choice</span>
          </div>
          <p>
            JARVIS uses your name and retrieves relevant profile sections for text and voice
            conversations. Nothing here trains or changes the model.
          </p>
          <label className="memory-toggle">
            <input
              type="checkbox"
              checked={profile.enabled}
              disabled={locked}
              onChange={(event) => {
                setProfile({ ...profile, enabled: event.target.checked });
                setSaved(false);
              }}
            />
            Use personal memory in future replies
          </label>
          <label htmlFor="profile-name">Your name</label>
          <Input
            id="profile-name"
            value={profile.display_name}
            maxLength={120}
            disabled={locked}
            onChange={(event) => {
              setProfile({ ...profile, display_name: event.target.value });
              setSaved(false);
            }}
          />
          <label htmlFor="profile-content">About you</label>
          <Textarea
            id="profile-content"
            value={profile.content}
            maxLength={20_000}
            disabled={locked}
            placeholder="Background, work, skills and preferences. Separate sections with a blank line."
            onChange={(event) => {
              setProfile({ ...profile, content: event.target.value });
              setSaved(false);
            }}
          />
          <small>
            {profile.content.length.toLocaleString()} / 20,000 characters · Separate sections with a
            blank line.
          </small>
          <div className="flex flex-wrap gap-2">
            <Button disabled={locked} onClick={() => void save(profile)}>
              {pending ? 'Please wait…' : 'Save memory'}
            </Button>
            <Button
              variant="outline"
              disabled={locked || (!profile.content && !profile.display_name)}
              onClick={() => setForget(true)}
            >
              <Trash2 size={15} />
              Forget profile
            </Button>
          </div>
          {forget && (
            <div className="memory-confirm">
              <p>
                Forget this profile? Future replies will no longer use it. Conversations are kept.
              </p>
              <Button variant="destructive" disabled={locked} onClick={() => void save(empty)}>
                Confirm forget
              </Button>
              <Button variant="ghost" onClick={() => setForget(false)}>
                Cancel
              </Button>
            </div>
          )}
          {saved && (
            <p role="status" className="voice-ready">
              Personal memory saved for future replies.
            </p>
          )}
          {error && (
            <p role="alert" className="voice-error">
              {error}
            </p>
          )}
          {disabled && <small>Finish the current chat or voice session to edit your memory.</small>}
          <small>
            Your profile is stored in local SQLite, which is not encrypted. Relevant facts go only
            to the configured LM Studio server on this device. Forgetting removes the active
            profile, not past chat messages or forensic disk traces.
          </small>
        </div>
      </SheetContent>
    </Sheet>
  );
}
