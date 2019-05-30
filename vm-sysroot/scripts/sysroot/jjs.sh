cd ../devtool
if [ "x$SKIP_DEVTOOL_PKG" == x ]
then cargo run -- pkg >&2
fi
cd ../pkg/ar_data/bin
ldd * | grep ' => ' | sed 's/^.* => //g' | sed 's/ (0x[0-9a-f]*)$//g'
