#!/bin/bash -x

cd ~/rust-axum

systemctl --user stop rust-axum.service

git pull -v

time cargo build -v --release
RESULT=$?
if [ $RESULT -ne 0 ]; then
  echo "cargo build failed RESULT = $RESULT"
  exit $RESULT
fi

systemctl --user restart rust-axum.service
