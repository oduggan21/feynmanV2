import { Toaster } from "@revlentless/ui/components/sonner";
import { Outlet } from "react-router";

export default function SessionLayout() {
  return (
    <div className="min-h-[100svh]">
      <Outlet />
      <Toaster richColors position="bottom-right" />
    </div>
  );
}
