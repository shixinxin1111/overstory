import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { applyTheme, readTheme } from './shared/theme';
import './styles/global.css';
applyTheme(readTheme());
createRoot(document.getElementById('root')!).render(<App/>);
