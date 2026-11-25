import type { CSSProperties } from 'react';
import { symbols } from './sf-symbols';

export type IconName = keyof typeof symbols;
export function Icon({ name, size = 18, style }: { name: IconName; size?: number; style?: CSSProperties }) {
  return <svg className="ui-icon" width={size} height={size} viewBox="0 0 24 24"
    fill="currentColor" aria-hidden="true" style={style}>
    <g transform="scale(.0625)">{symbols[name].map((path, index) =>
      <path key={index} d={path.d} transform={path.transform}/>
    )}</g>
  </svg>;
}
