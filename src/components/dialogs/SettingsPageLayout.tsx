import { Button } from "@fluentui/react-components";
import type { ReactNode } from "react";

type SettingsPage = {
  id: string;
  label: string;
};

type Props = {
  activePage: string;
  pages: SettingsPage[];
  onPageChange: (page: string) => void;
  children: ReactNode;
  styles: Record<string, string>;
};

export function SettingsPageLayout(props: Props) {
  return (
    <div className={props.styles.settingsLayout}>
      <nav className={props.styles.settingsNav} aria-label="设置分类">
        {props.pages.map((page) => (
          <Button
            key={page.id}
            className={props.styles.settingsNavButton}
            appearance={props.activePage === page.id ? "primary" : "secondary"}
            onClick={() => props.onPageChange(page.id)}
          >
            {page.label}
          </Button>
        ))}
      </nav>
      <div className={props.styles.settingsContent}>{props.children}</div>
    </div>
  );
}
