#!/bin/sh
set -e
id tester >/dev/null 2>&1 || useradd -M -s /usr/sbin/nologin tester
(echo "secret"; echo "secret") | smbpasswd -s -a tester
mkdir -p /data
exec smbd --foreground --no-process-group --debug-stdout
