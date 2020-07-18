sudo -u bennet git pull
docker build -t submission .
docker stop submission
docker rm submission
submission-system/run.sh
