import { invoke } from "@tauri-apps/api/core";
import type {
  BranchOperationResult,
  BranchSummary,
} from "../types/branches";

export interface BranchService {
  listBranches(repositoryPath: string): Promise<BranchSummary[]>;
  createBranch(repositoryPath: string, name: string): Promise<BranchOperationResult>;
  checkoutBranch(repositoryPath: string, name: string): Promise<BranchOperationResult>;
  deleteBranch(repositoryPath: string, name: string): Promise<BranchOperationResult>;
}

const mutationRequest = (repositoryPath: string, name: string) => ({
  request: { repositoryPath, name },
});

export const tauriBranchService: BranchService = {
  listBranches(repositoryPath) {
    return invoke<BranchSummary[]>("list_branches", {
      request: { repositoryPath },
    });
  },

  createBranch(repositoryPath, name) {
    return invoke<BranchOperationResult>(
      "create_branch",
      mutationRequest(repositoryPath, name),
    );
  },

  checkoutBranch(repositoryPath, name) {
    return invoke<BranchOperationResult>(
      "checkout_branch",
      mutationRequest(repositoryPath, name),
    );
  },

  deleteBranch(repositoryPath, name) {
    return invoke<BranchOperationResult>(
      "delete_branch",
      mutationRequest(repositoryPath, name),
    );
  },
};
