# Tant Terminal Shell Integration for Zsh
# This script provides Warp-style command block detection using OSC 133 sequences

# Only run if we're in an interactive shell
[[ -o interactive ]] || return 0

# Check if already loaded to prevent double-loading
[[ -n "$TANT_SHELL_INTEGRATION" ]] && return 0
export TANT_SHELL_INTEGRATION=1

# OSC 133 sequences for shell integration
# A = prompt start
# B = prompt end (command line start)
# C = command start (pre-execution)
# D = command end (pre-prompt, includes exit code)

# Save the original PROMPT if not already saved
[[ -z "$TANT_ORIGINAL_PS1" ]] && TANT_ORIGINAL_PS1="$PS1"

# Function to emit OSC sequences
_tant_osc() {
    printf "\033]133;%s\007" "$1"
}

# Emit OSC 7 current directory sequence
_tant_osc7_cwd() {
    printf "\033]7;file://%s%s\007" "$HOST" "$PWD"
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

# Pre-command hook: emitted before command execution
_tant_preexec() {
    # Emit command start marker
    _tant_osc "C"
}

# Pre-prompt hook: emitted before each prompt
_tant_precmd() {
    local exit_code=$?

    # Emit current directory (OSC 7)
    _tant_osc7_cwd
    
    # Emit command end marker with exit code
    _tant_osc "D;$exit_code"
    
    # Emit prompt start marker
    _tant_osc "A"

    # Emit git info after prompt start (updates on directory change)
    _tant_emit_git_info
}

# Post-prompt hook: emitted after prompt is displayed, before reading command
_tant_prompt_end() {
    # Emit prompt end marker (command line starts)
    _tant_osc "B"
}

# Set up hooks
autoload -Uz add-zsh-hook
add-zsh-hook preexec _tant_preexec
add-zsh-hook precmd _tant_precmd

# Modify PS1 to emit prompt end marker
# The %{ %} prevents the escape sequence from being counted in prompt width
PS1="%{$(_tant_osc 'A')%}${TANT_ORIGINAL_PS1}%{$(_tant_osc 'B')%}"

# Alternative: use PROMPT_COMMAND style
# This runs after precmd but before prompt display
if [[ -z "$TANT_USE_SIMPLE_PROMPT" ]]; then
    # Use a function to handle the prompt with markers
    _tant_set_prompt() {
        # Don't add markers if already present
        PS1="${TANT_ORIGINAL_PS1}"
    }
    add-zsh-hook precmd _tant_set_prompt
fi

# Print initial prompt markers for the first prompt
_tant_osc "A"

echo "Tant shell integration loaded for zsh"
