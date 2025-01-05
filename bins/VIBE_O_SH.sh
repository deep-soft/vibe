#!/bin/bash

export os_name=$("uname");
case $os_name in
  Darwin*)
    echo "Run on OSX";
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

if [[ $SED_EXE == '' ]]; then
  export SED_EXE="$(which sed)";
fi

echo "sed_exe _ $SED_EXE _";
rrules_sh=VIBE_O_SH.sh;
VIBE_O_HL="VIBE_O_HL";
rrules_in=$VIBE_O_HL.txt;
rrules=$VIBE_O_HL.sr;
echo prepare commands for $SED_EXE from input file $rrules_in;
echo $rrules_in;
$SED_EXE -e 's/^/s|/' -e $'s/\t/|/' -e 's/$/|/' $rrules_in > $rrules
echo extract string to search from $rrules_in;
awk_s=$(awk '{print $1}' $rrules_in);
echo $awk_s;

echo "Begin replace in files. Press ENTER to continue.";
#read n
f_count=0;
all_count=0;
for to_search in $awk_s;
do
  printf '\r\n#(_%s_)\r\n' $to_search;
  ((all_count++));
#  f_found=$(grep -rl --exclude-dir=.git --exclude=Cargo.* --exclude=*.lock --exclude=*.sh --exclude=*.sr $to_search . | tr '\r' ' ' | tr '\n' ' ')
  f_found=$(grep -rl --exclude-dir=.git --exclude=Cargo.* --exclude=*.lock --exclude=$rrules_in --exclude=$rrules $to_search . | tr '\r' ' ' | tr '\n' ' ');
  if [ ${#f_found} -ge 2 ]; then
#  if [[ ! $f_found == '' ]]; then
    $SED_EXE -i -f $rrules $f_found;
    printf '@(_%s_)\r\n' $f_found;
#    printf '\r\n##%s @@%s\r\n' $to_search $f_found
    ((f_count++));
  fi
done
echo Found/Total: $f_count / $all_count;
echo Done.;
if [[ $SED_EXE == '' ]]; then
  echo "some error!";
  exit 1;
fi
