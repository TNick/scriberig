/** @file Defines the responsive field-atlas shell for the integrated workbench. */
import {
  AppBar,
  Box,
  Chip,
  Divider,
  Drawer,
  FormControl,
  IconButton,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Select,
  Toolbar,
  Typography,
  useMediaQuery,
  useTheme,
} from "@mui/material";
import { useState, type ReactElement, type ReactNode } from "react";
import { pathnameForRoute, type AppRoute } from "./routes";

export interface AppearanceSettings {
  /** Density used for layout spacing. */
  density: "compact" | "comfortable" | "spacious";
  /** Base text scale for the application. */
  textSize: "small" | "medium" | "large";
  /** Explicit or operating-system-selected color scheme. */
  themeMode: "light" | "dark" | "system";
}

export const navigationGroups = [
  {
    title: "Workspace",
    routes: [
      { id: "projects", label: "Projects", icon: "▣" },
      { id: "artifacts", label: "Artifacts", icon: "▤" },
    ],
  },
  {
    title: "Explore",
    routes: [
      { id: "semantic", label: "Semantic", icon: "✳" },
      { id: "raw", label: "Knowledge tree", icon: "⌘" },
      { id: "document", label: "Documents", icon: "▧" },
      { id: "cad", label: "CAD drawings", icon: "⌗" },
      { id: "spreadsheet", label: "Spreadsheets", icon: "▦" },
      { id: "geo", label: "Maps & places", icon: "⌖" },
    ],
  },
  {
    title: "Review",
    routes: [
      { id: "projection", label: "Projections", icon: "◫" },
      { id: "history", label: "History", icon: "◷" },
      { id: "session", label: "Sessions", icon: "◉" },
      { id: "migration", label: "Changes", icon: "⇄" },
      { id: "operations", label: "Operations", icon: "⚙" },
    ],
  },
];

/** Renders a theme-aware density glyph using the current primary color.
 * @param props Density represented by the glyph.
 * @returns A high-contrast visual spacing indicator.
 */
function SpacingGlyph({ density }: { density: AppearanceSettings["density"] }): ReactElement {
  const widths =
    density === "compact" ? [8, 12, 16] : density === "comfortable" ? [10, 15, 20] : [12, 18, 24];
  return (
    <Box
      aria-hidden
      sx={{
        width: 26,
        height: 22,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        alignItems: "flex-start",
        gap: density === "compact" ? 0.25 : density === "comfortable" ? 0.5 : 0.75,
      }}
    >
      {widths.map((width, index) => (
        <Box key={index} sx={{ width, height: 2, borderRadius: 2, bgcolor: "primary.main" }} />
      ))}
    </Box>
  );
}

/** Renders grouped navigation for one available workbench route.
 * @param props Active route, navigation callback, and drawer close handler.
 * @returns Accessible grouped route navigation.
 */
function Navigation({
  active,
  onNavigate,
  onClose,
}: {
  active: string;
  onNavigate: (route: string) => void;
  onClose: () => void;
}): ReactElement {
  return (
    <Box component="nav" aria-label="Main navigation" sx={{ width: 256, py: 2 }}>
      <Box sx={{ px: 2.5, pb: 1.5 }}>
        <Typography variant="overline" color="text.secondary">
          Knowledge workspace
        </Typography>
      </Box>
      {navigationGroups.map((group) => (
        <Box key={group.title} sx={{ mb: 1.5 }}>
          <Typography variant="overline" color="text.secondary" sx={{ px: 2.5 }}>
            {group.title}
          </Typography>
          <List dense disablePadding>
            {group.routes.map((route) => (
              <ListItemButton
                component="a"
                key={route.id}
                href={pathnameForRoute(route.id as AppRoute)}
                selected={active === route.id}
                aria-current={active === route.id ? "page" : undefined}
                onClick={(event) => {
                  if (
                    event.button !== 0 ||
                    event.metaKey ||
                    event.ctrlKey ||
                    event.shiftKey ||
                    event.altKey
                  )
                    return;
                  event.preventDefault();
                  onNavigate(route.id);
                  onClose();
                }}
                sx={{
                  mx: 1,
                  borderRadius: 1.5,
                  minHeight: 40,
                  "&.Mui-selected": {
                    color: "primary.dark",
                    bgcolor: "primary.light",
                    "&:hover": { bgcolor: "primary.light" },
                  },
                }}
              >
                <ListItemIcon
                  aria-hidden
                  sx={{
                    minWidth: 32,
                    color: active === route.id ? "primary.dark" : "text.secondary",
                    fontSize: 18,
                    fontWeight: 700,
                  }}
                >
                  {route.icon}
                </ListItemIcon>
                <ListItemText primary={route.label} />
              </ListItemButton>
            ))}
          </List>
        </Box>
      ))}
    </Box>
  );
}

/** Renders navigation, project context, and the active workbench view.
 * @param props Current route, navigation callback, connection, workspace, and view content.
 * @returns The responsive application shell.
 */
export function AppShell({
  active,
  onNavigate,
  children,
  status,
  workspace,
  appearance,
  onAppearanceChange,
  helper,
}: {
  active: string;
  onNavigate: (route: string) => void;
  children: ReactNode;
  status: string;
  workspace?: { project: { name: string }; snapshot: string };
  appearance: AppearanceSettings;
  onAppearanceChange: (next: AppearanceSettings) => void;
  helper: ReactNode;
}): ReactElement {
  const theme = useTheme();
  const wide = useMediaQuery(theme.breakpoints.up("md"));
  const [menuOpen, setMenuOpen] = useState(false);
  const [appearanceAnchor, setAppearanceAnchor] = useState<HTMLElement | null>(null);
  const activeLabel =
    navigationGroups.flatMap((group) => group.routes).find((route) => route.id === active)?.label ??
    "Projects";
  const nav = (
    <Navigation active={active} onNavigate={onNavigate} onClose={() => setMenuOpen(false)} />
  );
  return (
    <Box sx={{ minHeight: "100vh", bgcolor: "background.default" }}>
      <AppBar
        position="sticky"
        elevation={0}
        color="inherit"
        sx={{ borderBottom: 1, borderColor: "divider" }}
      >
        <Toolbar sx={{ minHeight: 68, gap: 1.5 }}>
          {!wide && (
            <IconButton aria-label="Open navigation" onClick={() => setMenuOpen(true)}>
              ☰
            </IconButton>
          )}
          <Box sx={{ display: "flex", alignItems: "center", gap: 1.2, flexGrow: 1 }}>
            <Box
              aria-hidden
              sx={{
                width: 28,
                height: 28,
                borderRadius: "9px 9px 9px 3px",
                bgcolor: "primary.main",
                transform: "rotate(-8deg)",
              }}
            />
            <Typography variant="h6" sx={{ fontWeight: 750, letterSpacing: "-0.04em" }}>
              scriberig
            </Typography>
            <Divider orientation="vertical" flexItem sx={{ mx: 1 }} />
            <Typography variant="body2" color="text.secondary">
              {workspace?.project.name ?? "Knowledge workbench"}
            </Typography>
          </Box>
          <Chip
            size="small"
            label={status === "connected" ? "Connected" : status}
            color={status === "connected" ? "success" : "warning"}
            variant="outlined"
          />
          <IconButton
            aria-label="Appearance settings"
            aria-haspopup="true"
            aria-expanded={appearanceAnchor !== null}
            onClick={(event) => setAppearanceAnchor(event.currentTarget)}
          >
            ◐
          </IconButton>
          <Menu
            anchorEl={appearanceAnchor}
            open={appearanceAnchor !== null}
            onClose={() => setAppearanceAnchor(null)}
            slotProps={{ paper: { sx: { p: 2, width: 312 } } }}
          >
            <Typography variant="subtitle2" sx={{ mb: 1.5 }}>
              Appearance
            </Typography>
            <FormControl fullWidth size="small" sx={{ mb: 1.5 }}>
              <Select
                aria-label="Theme"
                value={appearance.themeMode}
                onChange={(event) =>
                  onAppearanceChange({
                    ...appearance,
                    themeMode: event.target.value as AppearanceSettings["themeMode"],
                  })
                }
              >
                <MenuItem value="system">🖥️ Theme: System</MenuItem>
                <MenuItem value="light">☀️ Theme: Light</MenuItem>
                <MenuItem value="dark">🌙 Theme: Dark</MenuItem>
              </Select>
            </FormControl>
            <FormControl fullWidth size="small" sx={{ mb: 1.5 }}>
              <Select
                aria-label="Spacing"
                value={appearance.density}
                renderValue={(value) => {
                  const density = value as AppearanceSettings["density"];
                  const label = {
                    compact: "Compact",
                    comfortable: "Comfortable",
                    spacious: "Spacious",
                  }[density];
                  return (
                    <Box
                      sx={{ display: "flex", alignItems: "center", gap: 1, whiteSpace: "nowrap" }}
                    >
                      <SpacingGlyph density={density} />
                      <Typography component="span" variant="body2">
                        Spacing: {label}
                      </Typography>
                    </Box>
                  );
                }}
                onChange={(event) =>
                  onAppearanceChange({
                    ...appearance,
                    density: event.target.value as AppearanceSettings["density"],
                  })
                }
              >
                <MenuItem value="compact">
                  <SpacingGlyph density="compact" />
                  Spacing: Compact
                </MenuItem>
                <MenuItem value="comfortable">
                  <SpacingGlyph density="comfortable" />
                  Spacing: Comfortable
                </MenuItem>
                <MenuItem value="spacious">
                  <SpacingGlyph density="spacious" />
                  Spacing: Spacious
                </MenuItem>
              </Select>
            </FormControl>
            <FormControl fullWidth size="small">
              <Select
                aria-label="Text size"
                value={appearance.textSize}
                onChange={(event) =>
                  onAppearanceChange({
                    ...appearance,
                    textSize: event.target.value as AppearanceSettings["textSize"],
                  })
                }
              >
                <MenuItem value="small">ᴀA Text size: Small</MenuItem>
                <MenuItem value="medium">Aa Text size: Medium</MenuItem>
                <MenuItem value="large">𝐀A Text size: Large</MenuItem>
              </Select>
            </FormControl>
          </Menu>
        </Toolbar>
      </AppBar>
      <Box sx={{ display: "flex", minHeight: "calc(100vh - 68px)" }}>
        {wide && (
          <Box
            sx={{
              width: 256,
              flexShrink: 0,
              borderRight: 1,
              borderColor: "divider",
              bgcolor: "background.paper",
            }}
          >
            {nav}
          </Box>
        )}
        <Drawer
          open={menuOpen}
          onClose={() => setMenuOpen(false)}
          sx={{ "& .MuiDrawer-paper": { width: 272 } }}
        >
          {nav}
        </Drawer>
        <Box
          component="main"
          sx={{ flex: 1, minWidth: 0, px: { xs: 2, sm: 3, lg: 5 }, py: { xs: 2.5, md: 4 } }}
        >
          <Box sx={{ maxWidth: 1440, mx: "auto" }}>
            <Box
              sx={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                mb: 2.5,
                gap: 2,
              }}
            >
              <Box>
                <Typography variant="overline" color="text.secondary">
                  {workspace ? `Snapshot ${workspace.snapshot}` : "Scriberig workbench"}
                </Typography>
                <Typography variant="h4" component="h1">
                  {activeLabel}
                </Typography>
              </Box>
              <Chip
                size="small"
                label={workspace ? "Workspace open" : "No project open"}
                variant="outlined"
              />
            </Box>
            {children}
            {helper}
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
