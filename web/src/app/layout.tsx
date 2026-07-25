import type { Metadata } from "next";
import { AuthGate } from "@/components/AuthGate";
import "@/styles/globals.css";

export const metadata: Metadata = {
  title: "Network Observatory",
  description: "Real-time network interface observability dashboard",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <AuthGate>{children}</AuthGate>
      </body>
    </html>
  );
}
