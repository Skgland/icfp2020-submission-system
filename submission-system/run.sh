#!/bin/bash

docker run -d \
 --rm \
 -v /var/run/docker.sock:/var/run/docker.sock \
 -p 8080:80 \
 submission
