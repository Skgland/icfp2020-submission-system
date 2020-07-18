#!/bin/bash

docker run -d \
 --name submission \
 --restart unless-stopped \
 -v /var/run/docker.sock:/var/run/docker.sock \
 -p 8080:80 \
 submission
