#!/bin/bash

docker run -d \
 --name submission \
 --restart unless-stopped \
 --network gitlab-network \
 --network-alias submission.local \
 -v /var/run/docker.sock:/var/run/docker.sock \
 -p 8080:80 \
 submission
