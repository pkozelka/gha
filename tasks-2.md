# Task 2 - Synchronous and asynchronous execution

Currently, the commmand `gha spawn` is asynchronous. It will dispatch the workflow and then exit immediately, without waiting for the workflow to complete.
We need to add a new option `--await` to the `gha spawn` command, which will make it wait for the workflow to complete before exiting. 
The command should exit with a non-zero status code if the workflow run fails.
Also there must be a new command, named `gha await`, which will wait for a workflow to complete and print the final status, using the run id provided by the asynchronous execution.
Organize the code in such a way that the logic for waiting for a workflow run to complete can be reused by both `gha spawn --await` and `gha await`.
Make sure that the exit code indicates whether the workflow run succeeded or failed, and that the final status is printed in a clear and user-friendly way.

The last INFO level message should contain Github UI URL to the workflow run.

The exit code should allow to distinguish at least between:
- workflow that failed to start,
- workflow that was canceled,
- workflow that succeeded,
- workflow that failed.

Where possible, use commonly used exit codes for each case.
