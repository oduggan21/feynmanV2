import {
  type RouteConfig,
  index,
  layout,
  route,
} from "@react-router/dev/routes";

export default [
  index("./routes/home.tsx"),
  layout("./layouts/app-shell.tsx", [
    route("dashboard", "./routes/dashboard.tsx"),
    route("sessions", "./routes/sessions-list.tsx"),
    route("sessions/new", "./routes/new-session.tsx"),
    route("settings", "./routes/settings.tsx"),
  ]),
  layout("./layouts/session-layout.tsx", [
    route("sessions/:id", "./routes/session-detail.tsx"),
  ]),
] satisfies RouteConfig;
