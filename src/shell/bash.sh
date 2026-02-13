z() {
    if [ $# -eq 0 ]; then
        cd ~
        return
    fi

    case "$1" in
        -x)
            shift
            if [ $# -eq 0 ]; then
                atuin-z -x -- "$PWD"
            else
                atuin-z -x -- "$@"
            fi
            return
            ;;
        -l|-h|--help)
            ATUIN_Z_PWD="$PWD" atuin-z "$@"
            return
            ;;
    esac

    local result
    result="$(ATUIN_Z_PWD="$PWD" atuin-z "$@")"
    if [ -n "$result" ]; then
        cd "$result"
    fi
}
