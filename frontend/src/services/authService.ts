import { invoke } from "@tauri-apps/api/core";
import type { Account, AuthStatus } from "../types/auth";

export interface AuthService {
  getStatus(): Promise<AuthStatus>;
  login(): Promise<Account>;
  cancelLogin(): Promise<void>;
  logout(): Promise<void>;
}

export const tauriAuthService: AuthService = {
  getStatus() {
    return invoke<AuthStatus>("get_auth_status");
  },

  login() {
    return invoke<Account>("login_github");
  },

  async cancelLogin() {
    await invoke<null>("cancel_github_login");
  },

  async logout() {
    await invoke<null>("logout_github");
  },
};
