import { flushAll, setSession } from '../../app/session';
import { mutate } from '../snapshot-store';
import type { Item, ModuleId } from '../types';
import { IconButton, run } from './Controls';

export function FavoriteButton({ module, item, className = '', closeEditor = false }: {
  module: ModuleId; item: Item; className?: string; closeEditor?: boolean;
}) {
  return <IconButton icon={item.favorited ? 'favoriteFill' : 'favorite'}
    label={`${item.favorited ? '取消收藏' : '收藏'}：${item.title}`}
    aria-pressed={!!item.favorited} className={`favorite-button ${className}`}
    onClick={() => void run(async () => {
      await flushAll();
      await mutate(module, { type: 'set_favorite', id: item.id, favorited: !item.favorited });
      if (closeEditor && item.favorited) setSession({ noteEditor: false });
    })}/>;
}
