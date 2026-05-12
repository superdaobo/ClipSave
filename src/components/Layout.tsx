import { ReactNode } from 'react';
import { NavLink } from 'react-router-dom';
import { Home, Clock, Settings, Info, Bug } from 'lucide-react';
import { t } from '../lib/i18n';

interface LayoutProps {
  children: ReactNode;
}

const navItems = [
  { to: '/', icon: Home, labelKey: 'nav.home' },
  { to: '/history', icon: Clock, labelKey: 'nav.history' },
  { to: '/settings', icon: Settings, labelKey: 'nav.settings' },
  { to: '/about', icon: Info, labelKey: 'nav.about' },
  { to: '/debug', icon: Bug, labelKey: 'nav.debug' },
];

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900 flex flex-col lg:flex-row">
      {/* Desktop Sidebar (≥1024px) */}
      <aside className="hidden lg:flex flex-col w-60 border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4">
        <div className="mb-8">
          <h1 className="text-xl font-bold text-gray-900 dark:text-white">
            {t('app.name')}
          </h1>
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
            {t('app.subtitle')}
          </p>
        </div>
        <nav className="flex flex-col gap-1" aria-label="Main navigation">
          {navItems.map(({ to, icon: Icon, labelKey }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive
                    ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 font-medium'
                    : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
                }`
              }
              aria-label={t(labelKey)}
            >
              <Icon className="w-5 h-5" />
              {t(labelKey)}
            </NavLink>
          ))}
        </nav>
      </aside>

      {/* Tablet Top Nav (640–1023px) */}
      <header className="hidden sm:flex lg:hidden items-center gap-4 px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
        <h1 className="text-lg font-bold text-gray-900 dark:text-white mr-auto">
          {t('app.name')}
        </h1>
        <nav className="flex gap-1" aria-label="Main navigation">
          {navItems.map(({ to, icon: Icon, labelKey }) => (
            <NavLink
              key={to}
              to={to}
              className={({ isActive }) =>
                `flex items-center gap-1 px-3 py-1.5 rounded-lg text-sm transition-colors ${
                  isActive
                    ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400'
                    : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
                }`
              }
              aria-label={t(labelKey)}
            >
              <Icon className="w-4 h-4" />
              <span className="hidden md:inline">{t(labelKey)}</span>
            </NavLink>
          ))}
        </nav>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto pb-16 sm:pb-0">
        {children}
      </main>

      {/* Mobile Bottom Tab Bar (<640px) */}
      <nav
        className="sm:hidden fixed bottom-0 left-0 right-0 bg-white dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 flex justify-around py-2 z-50"
        aria-label="Main navigation"
      >
        {navItems.map(({ to, icon: Icon, labelKey }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex flex-col items-center gap-0.5 px-3 py-1 text-xs transition-colors ${
                isActive
                  ? 'text-blue-600 dark:text-blue-400'
                  : 'text-gray-500 dark:text-gray-400'
              }`
            }
            aria-label={t(labelKey)}
          >
            <Icon className="w-5 h-5" />
            <span>{t(labelKey)}</span>
          </NavLink>
        ))}
      </nav>
    </div>
  );
}
