#!/bin/sh

YELLOW="\033[1;33m"
GREEN="\033[0;32m"
RED="\033[0;31m"
# Remove coloring
NO_COLOR="\033[0m"

# ddlog directories
DDLOG_INPUT_FILE="ddlog/typecheck.dl"
DDLOG_LIBRARY_DIR="ddlog"
DDLOG_OUTPUT_DIR="."

error() {
    local message="$1"

    if [ "$no_color" = "true" ]; then
        printf "error: $message"
    else
        printf "${RED}error:${NO_COLOR} $message"
    fi
}

warn() {
    local message="$1"

    if [ "$no_color" = "true" ]; then
        printf "warning: $message"
    else
        printf "${YELLOW}warning:${NO_COLOR} $message"
    fi
}

success() {
    local message="$1"

    if [ "$no_color" = "true" ]; then
        printf "$message"
    else
        printf "${GREEN}${message}${NO_COLOR}"
    fi
}

failure() {
    local message="$1"

    if [ "$no_color" = "true" ]; then
        printf "$message"
    else
        printf "${RED}${message}${NO_COLOR}"
    fi
}

check_subcommand() {
    local cmd_name="$1"

    if [ ! -z "$subcommand" ]; then
        error "the subcommand '$cmd_name' was given twice\n"
        exit 101
    fi
}

check_undeclared() {
    local var="$1"
    local arg_name="$2"

    if [ ! -z $var ]; then
        warn "the flag '$arg_name' was given twice\n"
    fi
}

extra_args=""
for arg in "$@"; do
    if [ "$arg" = "--debug" ]; then
        check_undeclared "$debug_flag" "--debug"
        extra_args="$extra_args --output-internal-relations --output-input-relations=INPUT_"
        debug_flag="true"

    elif [ "$arg" = "--no-color" ]; then
        check_undeclared "$no_color" "--no-color"
        no_color="true"

    elif [ "$arg" = "--no-check" ]; then
        check_undeclared "$no_check" "--no-check"
        no_check="true"

    elif [ "$arg" = "--no-fmt" ]; then
        check_undeclared "$no_rustfmt" "--no-fmt"
        no_rustfmt="true"

    elif [ "$arg" = "--help" ] || [ "$arg" = "-h" ]; then
        printf "USAGE:\n"
        printf "    ./ddlog.sh [SUBCOMMAND] [FLAGS]\n"
        printf "\n"
        printf "FLAGS:\n"
        printf "    -h, --help              Display this help message\n"
        printf "        --no-check          Don't run 'cargo check' on generated code\n"
        printf "        --debug             Enable debug mode (causes ddlog to dump internal tables)\n"
        printf "        --no-color          Disable terminal coloring\n"
        printf "        --no-fmt            Don't run rustfmt"
        # TODO: Allow customizing these
        # printf "    -o, --output-dir        Set the output dir, defaults to $DDLOG_OUTPUT_DIR\n"
        # printf "    -i, --input-file        Set the input file, defaults to $DDLOG_INPUT_FILE\n"
        # printf "    -L, --library-dir       Set extra library search paths, defaults to $DDLOG_LIBRARY_DIR\n"
        printf "\n"
        printf "SUBCOMMANDS (defaults to 'compile'):\n"
        printf "    compile     Compile ddlog source into rust\n"
        printf "    check       Check that the ddlog source is valid\n"
        printf "\n"

        exit 0
    
    elif [ "$arg" = "compile" ]; then
        check_subcommand "compile"
        subcommand="compile"

    elif [ "$arg" = "check" ]; then
        check_subcommand "check"
        subcommand="check"

    elif [ ! -z "$arg" ]; then
        error "unrecognized flag '$arg'\n"
        exit 101
    fi
done

if [ "$subcommand" = "check" ]; then
    if [ "$debug_flag" = "true" ]; then
        warn "'--debug' does nothing in check mode\n"
    fi

    printf "checking "
    compile_action="validate"

elif [ "$subcommand" = "compile" ] || [ -z "$subcommand" ]; then
    printf "compiling "
    compile_action="compile"

else
    error "unrecognized subcommand '$subcommand'\n"
    exit 101
fi

if [ "$debug_flag" = "true" ] && [ "$subcommand" != "check" ]; then
    printf "ddlog in debug mode... "
else
    printf "ddlog... "
fi

if ! [ "$no_rustfmt" = "true" ]; then
    extra_args="$extra_args --run-rustfmt"
fi

ddlog -i $DDLOG_INPUT_FILE \
      -L $DDLOG_LIBRARY_DIR \
      --action $compile_action \
      --output-dir=$DDLOG_OUTPUT_DIR \
      --omit-profile \
      --omit-workspace \
      $extra_args

exit_code=$?
if [ $exit_code -ne 0 ]; then
    failure "failed\n"
    exit $exit_code
else
    success "ok\n"
fi

if ( [ "$subcommand" = "compile" ] || [ -z "$subcommand" ] ) && [ "$no_check" != "true" ]; then
    cd typecheck_ddlog

    printf "checking generated code... "
    cargo --quiet check

    exit_code=$?
    if [ $exit_code -ne 0 ]; then
        failure "failed\n"
        exit $exit_code
    else
        success "ok\n"
    fi
fi
