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
