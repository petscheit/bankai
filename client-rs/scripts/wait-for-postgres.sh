#!/bin/bash

while ! nc -z localhost 5434; do sleep 1; done
echo "Postgres ready"
sleep 1
exit 0
