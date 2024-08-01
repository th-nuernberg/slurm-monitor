#!/bin/bash
# Courtesy of ChatJibbedy
# https://chatgpt.com/share/395eec58-30a6-407e-8ad7-89d5b9930003


# Enable strict error handling
set -euo pipefail

# Load environment variables from .env file
if [[ -f "deploy.env" ]]; then
    export $(grep -v '^#' deploy.env | xargs)
else
    echo "deploy.env file not found. Please create one based on deploy.env.example." >&2
    exit 1
fi

# Set variables
TMUX_SESSION_NAME="${TMUX_SESSION_NAME:-slurm_deploy}"
EMAIL_SUBJECT="${EMAIL_SUBJECT:-[slurm-monitor] Deployment Failure}"
LOG_FILE="${LOG_FILE:-/tmp/deploy.log}"
RUN_PARAMS="${RUN_PARAMS:-}"


SCRIPT_DIR="$(dirname "$(realpath "$0")")"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

# ---- Start of logic ----
# Convert comma-separated EMAIL_TO into a Bash array
IFS=',' read -ra EMAIL_TO_ARRAY <<< "$EMAIL_TO"

# Function to handle errors
handle_error() {
    local exit_code=$?
    local msg=${1:-"An unexpected error occurred."}
    echo "Error: $msg (Exit Code: $exit_code)" | tee -a "$LOG_FILE"
    send_email
    exit $exit_code
}

# Function to send email
send_email() {
    for email in "${EMAIL_TO_ARRAY[@]}"; do
        if ! mail -s "$EMAIL_SUBJECT" "$email" < "$LOG_FILE"; then
            echo "Failed to send email to $email." | tee -a "$LOG_FILE"
        fi
    done
}

# Function to check if a command exists
check_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        handle_error "Required command '$1' is not installed."
    fi
}

# Validate email format (basic regex)
validate_email() {
    for email in "${EMAIL_TO_ARRAY[@]}"; do
        if [[ ! "$email" =~ ^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]; then
            handle_error "Invalid email format: $email"
        fi
    done
}

# Trap errors and call the error handler
trap 'handle_error' ERR

# Check for required commands
check_command git
check_command cargo
check_command tmux
check_command mail

# Validate email
validate_email

# Ensure log file is clean
rm -f "$LOG_FILE"
touch "$LOG_FILE"

# Redirect stdout and stderr to both console and log file
exec > >(tee -a "$LOG_FILE") 2>&1

# Check if repository directory exists
if [[ ! -d "$REPO_DIR" ]]; then
    handle_error "Repository directory '$REPO_DIR' does not exist."
fi

# Change to the repository directory
cd "$REPO_DIR"

# Verify it's a git repository
if [[ ! -d ".git" ]]; then
    handle_error "Directory '$REPO_DIR' is not a Git repository."
fi

# Pull new changes from GitHub (--ff-only)
echo "Pulling latest changes from GitHub..."
git pull --ff-only origin main

# Rebuild using Cargo
echo "Rebuilding project..."
cargo build --release

# Kill old tmux session if it exists
if tmux has-session -t "$TMUX_SESSION_NAME" 2>/dev/null; then
    echo "Killing old tmux session..."
    tmux kill-session -t "$TMUX_SESSION_NAME"
fi

# Create new tmux session
echo "Creating new tmux session..."
tmux new-session -d -s "$TMUX_SESSION_NAME"

# Launch the server in the new tmux session
echo "Launching server in tmux session..."
tmux send-keys -t "$TMUX_SESSION_NAME" "cd $REPO_DIR && cargo run --release -- $RUN_PARAMS" C-m

echo "Deployment completed successfully!"
