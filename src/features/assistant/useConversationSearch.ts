import { useEffect, useState } from 'react';
import type { AssistantGateway } from '@/domain/ports';
import type { Conversation } from '@/domain/models';
import { errorText } from '@/lib/errors';

export function useConversationSearch(
  api: AssistantGateway,
  conversations: Conversation[],
  enabled: boolean,
) {
  const [query, setQuery] = useState('');
  const [matches, setMatches] = useState<Conversation[]>([]);
  const [searching, setSearching] = useState(false);
  const [settledTerm, setSettledTerm] = useState('');
  const [error, setError] = useState('');
  const term = query.trim();
  useEffect(() => {
    if (!term || !enabled) {
      setSearching(false);
      setError('');
      return;
    }
    let current = true;
    setSearching(true);
    setError('');
    setMatches([]);
    const timeout = window.setTimeout(() => {
      api
        .search(term)
        .then((results) => {
          if (current) setMatches(results);
        })
        .catch((error) => {
          if (current) setError(errorText(error));
        })
        .finally(() => {
          if (current) {
            setSettledTerm(term);
            setSearching(false);
          }
        });
    }, 250);
    return () => {
      current = false;
      window.clearTimeout(timeout);
    };
  }, [api, term, conversations, enabled]);
  return {
    query,
    setQuery,
    conversations: term ? (settledTerm === term ? matches : []) : conversations,
    searching: enabled && !!term && (searching || settledTerm !== term),
    error: settledTerm === term ? error : '',
  };
}
