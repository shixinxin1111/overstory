import type { Group } from './types';

export const orderedGroups = (groups: Group[]) => [
  ...groups.filter(group => group.id !== 'ungrouped'),
  ...groups.filter(group => group.id === 'ungrouped'),
];
