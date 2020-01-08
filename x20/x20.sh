# Create directories
mkdir -p ~/.x20/bin
mkdir -p ~/.x20/config
mkdir -p ~/.x20/logs
echo ✔️ Created ~/.x20 directories

curl -LsSf http://x20.colinmerkel.xyz/x20 --output ~/.x20/bin/x20
status=$?
if [ $status -ne 0 ]; then
   echo "❌Failed to download x20 binary!"
   exit 1
fi
echo ✔️ Downloaded x20 binary

chmod +x ~/.x20/bin/x20
status=$?
if [ $status -ne 0 ]; then
   echo "❌Failed to mark x20 binary as executable!"
   exit 1
fi
echo ✔️ Marked x20 binary as executable

~/.x20/bin/x20 update
status=$?
if [ $status -ne 0 ]; then
   echo "❌Failed to sync/update!"
   exit 1
fi
echo ✔️ Initialized x20

echo 🎉 Finished installation!
echo
echo You should add ~/.x20/bin to your path.
