dd if=/dev/random of=./setup.exe bs=1024 count=1000000
zip TheGame.zip setup.exe
rm setup.exe