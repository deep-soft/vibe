#!/bin/bash
export os_name=$("uname");
case $os_name in
  Darwin*)
    echo "Run on OSX";
    echo "Install gnu-sed";
    brew install gnu-sed;
    export PATH="/opt/homebrew/opt/gnu-sed/libexec/gnubin:$PATH";
    echo "SED_EXE=$(which sed)" >> $GITHUB_ENV;
    ls -l /opt/homebrew/opt/gnu-sed/libexec/gnubin || true;
  ;;
  Linux*)
    echo "Run on Linux";
    echo "SED_EXE=$(which sed)" >> $GITHUB_ENV;
    ;;
  *)
    echo "Run on unknown OS: $os_name";
    echo "SED_EXE=$(which sed)" >> $GITHUB_ENV;
  ;;
esac
