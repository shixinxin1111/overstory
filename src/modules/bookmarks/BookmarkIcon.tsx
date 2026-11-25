import { useState } from 'react';
import { Icon } from '../../shared/icons';

export function BookmarkIcon({ icon }: { icon?: string }) {
  const [failed, setFailed] = useState<string | null>(null);
  const source = icon?.trim();
  if (!source || source === failed || !/^https?:\/\//i.test(source)) return <Icon name="bookmarks"/>;
  return <img className="bookmark-icon" src={source} width={18} height={18} alt=""
    draggable={false} loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(source)}/>;
}
