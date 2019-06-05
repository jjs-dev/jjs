cd ../devtool
cd ../pkg/ar_data/bin
ldd * | grep ' => ' | sed 's/^.* => //g' | sed 's/ (0x[0-9a-f]*)$//g'
