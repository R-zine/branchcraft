import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AuthService } from "../../services/authService";
import type { Account, AuthStatus } from "../../types/auth";
import { AccountMenu } from "./AccountMenu";

const account: Account = {
  id: 42,
  login: "branchcrafter",
  name: "Branch Crafter",
  avatarUrl: "https://avatars.githubusercontent.com/u/42",
  profileUrl: "https://github.com/branchcrafter",
};

function service(status: AuthStatus) {
  const login = vi.fn<AuthService["login"]>().mockResolvedValue(account);
  const logout = vi.fn<AuthService["logout"]>().mockResolvedValue(undefined);
  return {
    authService: {
      getStatus: vi.fn().mockResolvedValue(status),
      login,
      cancelLogin: vi.fn().mockResolvedValue(undefined),
      logout,
    } satisfies AuthService,
    login,
    logout,
  };
}

describe("AccountMenu", () => {
  it("signs in through the service and displays the current user", async () => {
    const { authService, login } = service({ state: "signedOut" });
    render(<AccountMenu service={authService} />);

    fireEvent.click(screen.getByRole("button", { name: "GitHub account" }));
    fireEvent.click(await screen.findByRole("button", { name: "Sign in with GitHub" }));

    expect(
      await screen.findByRole("button", {
        name: "GitHub account branchcrafter",
      }),
    ).toBeDefined();
    expect(login).toHaveBeenCalledOnce();
  });

  it("logs out without exposing credentials to the UI", async () => {
    const { authService, logout } = service({ state: "signedIn", account });
    render(<AccountMenu service={authService} />);

    const trigger = await screen.findByRole("button", {
      name: "GitHub account branchcrafter",
    });
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    await waitFor(() => expect(logout).toHaveBeenCalledOnce());
    expect(await screen.findByText("Connect GitHub")).toBeDefined();
  });

  it("explains when OAuth is not configured for the build", async () => {
    const { authService } = service({ state: "unconfigured" });
    render(<AccountMenu service={authService} />);

    fireEvent.click(screen.getByRole("button", { name: "GitHub account" }));

    expect(await screen.findByText("GitHub sign-in unavailable")).toBeDefined();
  });
});
