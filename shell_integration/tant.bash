# Tant Terminal Shell Integration for Bash
# This script provides Warp-style command block detection using OSC 133 sequences

# Only run if we're in an interactive shell
[[ $- == *i* ]] || return 0

# Check if already loaded to prevent double-loading
[[ -n "$TANT_SHELL_INTEGRATION" ]] && return 0
export TANT_SHELL_INTEGRATION=1

# OSC 133 sequences for shell integration
# A = prompt start
# B = prompt end (command line start)
# C = command start (pre-execution)
# D = command end (pre-prompt, includes exit code)

# Save the original prompt if not already saved
[[ -z "$TANT_ORIGINAL_PS1" ]] && TANT_ORIGINAL_PS1="$PS1"

# Function to emit OSC sequences
_tant_osc() {
    printf "\033]133;%s\007" "$1"
}

# Emit git info (branch + status) using OSC 133;G
_tant_emit_git_info() {
    local git_info
    local git_status
    local branch
    git_info=$(git -C . status --porcelain=v2 -b 2>/dev/null) || return 0
    branch=$(printf "%s\n" "$git_info" | command awk '/^# branch.head /{print $3; exit}')
    [[ -z "$branch" || "$branch" == "(detached)" ]] && return 0

    if printf "%s\n" "$git_info" | command awk '($1 == "u" || $1 == "?" || $1 == "1" || $1 == "2") {exit 0} END {exit 1}'; then
        git_status="dirty"
    else
        git_status="clean"
    fi

    if printf "%s\n" "$git_info" | command awk '$1 == "u" {exit 0} END {exit 1}'; then
        git_status="conflicts"
    fi

    _tant_osc "G;branch=${branch};status=${git_status}"
}

# Pre-execution hook: runs right before command execution
_tant_preexec() {
    # Emit command start marker
    _tant_osc "C"
}

# Pre-prompt hook: runs before each prompt display
_tant_precmd() {
    local exit_code=$?
    
    # Emit command end marker with exit code
    _tant_osc "D;$exit_code"
    
    # Emit prompt start marker
    _tant_osc "A"

    # Emit git info after prompt start (updates on directory change)
    _tant_emit_git_info
}

# Set up DEBUG trap for preexec functionality
# This runs before each command
_tant_debug_trap() {
    # Only run preexec for actual commands, not for PROMPT_COMMAND
    if [[ "$BASH_COMMAND" != "$PROMPT_COMMAND" && "$BASH_COMMAND" != "_tant_precmd" ]]; then
        _tant_preexec
    fi
}

# Enable DEBUG trap
trap '_tant_debug_trap' DEBUG

# Set up PROMPT_COMMAND to run our precmd
if [[ -z "$PROMPT_COMMAND" ]]; then
    PROMPT_COMMAND="_tant_precmd"
else
    # Prepend to existing PROMPT_COMMAND
    PROMPT_COMMAND="_tant_precmd; $PROMPT_COMMAND"
fi

# Modify PS1 to include prompt end marker
# \[ \] prevents the escape sequence from being counted in prompt width
PS1="\[$(_tant_osc 'B')\]${TANT_ORIGINAL_PS1}"

# Emit initial prompt start for the first prompt
_tant_osc "A"

echo "Tant shell integration loaded for bash"
