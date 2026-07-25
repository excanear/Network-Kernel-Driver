"use client";

import { createContext, useContext, useEffect, useState } from "react";
import { getCurrentUser, login as apiLogin, logout as apiLogout, type CurrentUser } from "@/lib/api";
import { theme } from "@/styles/theme";

interface AuthContextValue {
  user: CurrentUser | null;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue>({ user: null, logout: async () => {} });

export function useAuth() {
  return useContext(AuthContext);
}

export function AuthGate({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<CurrentUser | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    getCurrentUser()
      .then(setUser)
      .catch(() => setUser(null))
      .finally(() => setLoading(false));
  };

  useEffect(refresh, []);

  const logout = async () => {
    await apiLogout();
    setUser(null);
  };

  if (loading) {
    return <div style={{ minHeight: "100vh", background: theme.page }} />;
  }

  if (!user) {
    return <LoginForm onSuccess={refresh} />;
  }

  return <AuthContext.Provider value={{ user, logout }}>{children}</AuthContext.Provider>;
}

function LoginForm({ onSuccess }: { onSuccess: () => void }) {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await apiLogin(username, password);
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: theme.page,
      }}
    >
      <form
        onSubmit={handleSubmit}
        style={{
          background: theme.surface,
          border: `1px solid ${theme.border}`,
          borderRadius: 8,
          padding: 32,
          width: 320,
        }}
      >
        <h1 style={{ fontSize: 18, color: theme.textPrimary, margin: "0 0 4px" }}>Network Observatory</h1>
        <p style={{ fontSize: 13, color: theme.textMuted, margin: "0 0 20px" }}>Sign in to continue</p>

        <label style={{ display: "block", fontSize: 12, color: theme.textMuted, marginBottom: 4 }}>Username</label>
        <input
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          style={inputStyle}
          autoComplete="username"
        />

        <label style={{ display: "block", fontSize: 12, color: theme.textMuted, margin: "12px 0 4px" }}>
          Password
        </label>
        <input
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          style={inputStyle}
          autoComplete="current-password"
        />

        {error && <p style={{ color: theme.status.critical, fontSize: 12, marginTop: 12 }}>{error}</p>}

        <button
          type="submit"
          disabled={submitting}
          style={{
            marginTop: 20,
            width: "100%",
            padding: "10px 0",
            background: theme.seriesRx,
            color: "#fff",
            border: "none",
            borderRadius: 6,
            cursor: "pointer",
            fontSize: 14,
          }}
        >
          {submitting ? "Signing in…" : "Sign in"}
        </button>

        <p style={{ fontSize: 11, color: theme.textMuted, marginTop: 16 }}>
          First run? Check the service log for the bootstrapped <code>admin</code> password.
        </p>
      </form>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "8px 10px",
  background: "#141414",
  border: `1px solid ${theme.border}`,
  borderRadius: 6,
  color: theme.textPrimary,
  fontSize: 13,
};
