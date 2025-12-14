use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use anyhow::{Context, Result, anyhow};
use crate::types::{Ticket, Status};
use crate::context::discovery::discover_context;
use crate::verification::visual_diff::verify_visual;
use std::fs;

pub struct ExecutionLoop<'a> {
    workspace_root: &'a Path,
    agent_cmd: String,
    ticket: Ticket,
}

impl<'a> ExecutionLoop<'a> {
    pub fn new(workspace_root: &'a Path, agent_cmd: String, ticket: Ticket) -> Self {
        Self {
            workspace_root,
            agent_cmd,
            ticket,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // 1. Safety Check: Ensure git is clean
        if self.is_git_dirty()? {
            return Err(anyhow!("Workspace is dirty. Please commit or stash changes before running execution loop."));
        }

        // 2. Detached HEAD
        self.enter_detached_head()?;

        let max_retries = self.ticket.verification.max_retries.unwrap_or(5);
        let mut attempts = 0;
        let mut previous_errors = Vec::new();
        let mut success = false;

        while attempts < max_retries {
            println!(">> Attempt {}/{}", attempts + 1, max_retries);

            // 3. Generate Prompt
            let prompt = self.generate_prompt(&previous_errors)?;

            // 4. Run Agent
            if let Err(e) = self.run_agent(&prompt) {
                // Agent failed to run (crashed). This is fatal or just an error?
                // PR says: "stderr should be captured if the agent itself crashes"
                // But generally "Feed the error log back ... for the next turn" applies to Verification failures.
                // If the agent crashes, maybe we should just retry with that info?
                previous_errors.push(format!("Agent Execution Failed: {}", e));
                attempts += 1;
                continue;
            }

            // 5. Verification
            match self.verify() {
                Ok(_) => {
                    success = true;
                    println!(">> Verification PASSED!");
                    break;
                }
                Err(e) => {
                    println!(">> Verification FAILED: {}", e);
                    previous_errors.push(format!("Verification Failed:\n{}", e));
                    attempts += 1;

                    // Revert changes for next attempt?
                    // PR says: "Agent writes files -> Verify -> Revert/Retry"
                    // Wait, if we revert, the agent starts from scratch?
                    // Usually "Feedback Loop" means the agent fixes the code.
                    // If we revert, the agent loses its work.
                    // Ah, "Revert/Retry" in "The Flow" usually means we revert to the snapshot state
                    // SO THAT the agent can try again from a clean slate OR the agent iteratively fixes it.
                    // The PR description says: "Expectation: See the logs show 1 failure ... followed by an Agent retry where it removes the non-existent function"
                    // This implies the agent modifies the state.
                    // If we revert every time, the agent has to re-apply everything + fix.
                    // Let's assume we DO NOT revert between attempts inside the loop, UNLESS the strategy is "try again from scratch".
                    // However, the PR says: "Loop: Feeds the error log back to the Agent as the only context for the next turn."
                    // And "The Flow: ... Revert/Retry".
                    // Given the "Agent guessing" context, maybe it IS iterative.
                    // But if I strictly follow "Revert/Retry" in the "Detached HEAD" section:
                    // "Run the Loop (Agent writes files -> Verify -> Revert/Retry)."
                    // If I revert, I lose the file change.
                    // If I don't revert, the agent sees the file change it just made.
                    // Usually with LLMs, you want them to see the broken code they just wrote and fix it.
                    // So I will NOT revert inside the loop. I will only revert if the ENTIRE loop fails.
                    // Wait, "Scenario A" says "Agent retry where it removes the non-existent function".
                    // This implies the file persisted.
                }
            }
        }

        if success {
            // Success: Commit the changes or leave them?
            // "Success: Commit the changes (or leave them staged) and git checkout -"
            // I'll leave them in the detached head state and let the user know?
            // Or I can try to merge back?
            // Since we are in detached HEAD, the changes are committed to that detached commit?
            // Wait, I haven't been committing inside the loop.
            // If I just modified files, they are in the working tree.
            // If I verify and it passes, the files are dirty in the detached HEAD.
            // If I `git checkout -`, I might carry them over if they don't conflict.
            // Or I can commit them to a new branch.

            // Let's just return success and tell the user they are in a detached HEAD with the fix.
            println!(">> Task Completed Successfully!");
            println!(">> You are currently in a detached HEAD state with the changes.");
            println!(">> To save, run: git checkout -b feature/completed-task-{}", self.ticket.meta.id);
            Ok(())
        } else {
            // Failure: Reset
            println!(">> Max retries reached. Reverting to original state.");
            self.reset_hard()?;
            self.leave_detached_head()?;
            Err(anyhow!("Task execution failed after {} retries.", max_retries))
        }
    }

    fn is_git_dirty(&self) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(self.workspace_root)
            .args(&["status", "--porcelain"])
            .output()?;
        Ok(!output.stdout.is_empty())
    }

    fn enter_detached_head(&self) -> Result<()> {
        Command::new("git")
            .current_dir(self.workspace_root)
            .args(&["checkout", "--detach"])
            .status()
            .context("Failed to enter detached HEAD")?;
        Ok(())
    }

    fn leave_detached_head(&self) -> Result<()> {
         // This tries to go back to the previous branch/commit
         Command::new("git")
            .current_dir(self.workspace_root)
            .arg("checkout")
            .arg("-")
            .status()
            .context("Failed to leave detached HEAD")?;
        Ok(())
    }

    fn reset_hard(&self) -> Result<()> {
        Command::new("git")
            .current_dir(self.workspace_root)
            .args(&["reset", "--hard"])
            .status()
            .context("Failed to hard reset")?;
        Ok(())
    }

    fn generate_prompt(&self, errors: &[String]) -> Result<String> {
        // Construct the context
        // If ticket.relevant_files is empty AND auto_context is true (or implied?), run discovery.
        // PR says "if relevant_files is empty in the TOML, the engine now dynamically populates context."
        // Also schema added `auto_context` boolean.

        let mut relevant_files = self.ticket.spec.relevant_files.clone();
        if relevant_files.is_empty() { // && self.ticket.spec.auto_context {
             // Logic says "if relevant_files is empty... dynamically populates"
             // I'll assume auto_context defaults to true if not specified?
             // Or explicitly check the flag. The flag was added to schema.
             // Let's check the flag.
             if self.ticket.spec.auto_context {
                 relevant_files = discover_context(&self.ticket, self.workspace_root);
             }
        }

        let mut context_content = String::new();
        for file in &relevant_files {
            let path = self.workspace_root.join(file);
            if path.exists() {
                context_content.push_str(&format!("--- FILE: {} ---\n", file));
                context_content.push_str(&fs::read_to_string(path).unwrap_or_default());
                context_content.push_str("\n\n");
            }
        }

        let mut prompt = String::new();
        prompt.push_str(&format!("# Task: {}\n\n", self.ticket.meta.title));
        prompt.push_str(&format!("## Description\n{}\n\n", self.ticket.spec.description));
        prompt.push_str(&format!("## Constraints\n{:?}\n\n", self.ticket.spec.constraints));

        if !context_content.is_empty() {
             prompt.push_str("# Context\n");
             prompt.push_str(&context_content);
        }

        if !errors.is_empty() {
            prompt.push_str("\n# Previous Errors (FIX THESE)\n");
            for err in errors {
                prompt.push_str(&format!("- {}\n", err));
            }
        }

        Ok(prompt)
    }

    fn run_agent(&self, prompt: &str) -> Result<()> {
        // Splitting the agent command string into program and args is tricky.
        // `sh -c` is safer to handle complex commands like "cursor --prompt".
        // But the requirements say: "The director-plan process will construct the full prompt... and stream it to the stdin of the command string provided"
        // And "Example: echo 'FULL_PROMPT' | cursor --stdin-mode"
        // So we should spawn the command and write to its stdin.

        // We'll use `sh -c` to execute the agent string so flags work.
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.agent_cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit()) // Log to user terminal
            .stderr(Stdio::piped()) // Capture stderr just in case, or inherit?
                                    // "stderr should be captured if the agent itself crashes"
            .spawn()
            .context("Failed to spawn agent command")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes())?;
        }

        let output = child.wait_with_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Agent exited with status {}: {}", output.status, stderr));
        }

        Ok(())
    }

    fn verify(&self) -> Result<()> {
        // 1. Run Verification Command
        let cmd_str = &self.ticket.verification.command;
        if !cmd_str.is_empty() {
             let output = Command::new("sh")
                .arg("-c")
                .arg(cmd_str)
                .current_dir(self.workspace_root)
                .output()
                .context("Failed to execute verification command")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Err(anyhow!("Command Failed:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr));
            }
        }

        // 2. Visual Verification
        if let Some(golden_image) = &self.ticket.verification.golden_image {
             let report = verify_visual(self.workspace_root, golden_image)?;
             if report.diff_detected {
                 return Err(anyhow!("Visual Verification Failed: {}\nDiff Bounds: {:?}\nReason: {:?}",
                    report.mismatch_percentage, report.diff_bounds, report.reason));
             }
        }

        Ok(())
    }
}
