function z
    if test (count $argv) -eq 0
        cd ~
        return
    end

    switch $argv[1]
        case -x
            set -e argv[1]
            if test (count $argv) -eq 0
                ATUIN_Z_PWD="$PWD" atuin-z -x -- $PWD
            else
                ATUIN_Z_PWD="$PWD" atuin-z -x -- $argv
            end
            return
        case -l -h --help
            ATUIN_Z_PWD="$PWD" atuin-z $argv
            return
    end

    set -l result (ATUIN_Z_PWD="$PWD" atuin-z $argv)
    if test -n "$result"
        cd $result
    end
end
