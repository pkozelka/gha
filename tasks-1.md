
# Task 1

Currently, executing `gha gen` creates a Makefile which allows the user to run the workflow.

Now we want to make it so that the user can run the workflow directly, using a command like `gha run <workflow-name> -b <branch> arg1=val1 arg2=val2...`.

The implementation should ensure that:
- contents of all HTTP requests and responses are properly logged at debug level
- headers are logged at trace level
- other significant information is logged at info level
- a support for bash/zsh completion is provided, for all the commands and their parameters; in case of arg type 'option', also the values must be supported
- github authentication is supported the same way as in the `gha gen` command: either via GITHUB_TOKEN environment variable or via ~/.netrc file,
  with additional option to specify the token via command line argument `--token <token>` (this should override the environment variable and netrc file if provided)
- code readability and maintainability is important, so the implementation should be well-structured and follow Rust best practices
- the implementation should be tested
- new code should be placed in separate files, with only minimal changes to the existing code
