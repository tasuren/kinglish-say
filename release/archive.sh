cd dist/release
for name in $(ls -d */); do
  zip -x "*.DS_Store" "*__MACOSX*" -FSr ${name:0:-1}.zip $name*
done