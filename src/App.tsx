import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Layout } from './components/Layout';
import { HomePage } from './routes/HomePage';
import { HistoryPage } from './routes/HistoryPage';
import { SettingsPage } from './routes/SettingsPage';
import { AboutPage } from './routes/AboutPage';
import { DebugPage } from './routes/DebugPage';
import { useTheme } from './hooks/useTheme';
import { useTaskEvents } from './hooks/useTaskEvents';

function App() {
  // Initialize theme on mount
  useTheme();
  // Listen for backend task events
  useTaskEvents();

  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/about" element={<AboutPage />} />
          <Route path="/debug" element={<DebugPage />} />
        </Routes>
      </Layout>
    </BrowserRouter>
  );
}

export default App;
