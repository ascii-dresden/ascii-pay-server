#/bin/bash


function onExit {
    if [ "$?" != "0" ]; then
        echo "Tests failed";
        echo;
        docker compose -f docker-compose.test.yml down -v;
        exit 1;
    else
        echo "Tests passed";
        echo;
        docker compose -f docker-compose.test.yml down -v;
    fi
}

trap onExit EXIT;

previous_command=`docker images | grep asciipay/newman`
cmd=$previous_command ret=$?
if [ $ret -ne 0 ]; then
    docker build -f docker/newman.Dockerfile -t asciipay/newman:latest .;
fi

# Stop on first error
set -e;

if [[ $* == *--build* ]]; then
    docker compose -f docker-compose.test.yml up -d --build;
else
    docker compose -f docker-compose.test.yml up -d;
fi

sleep 1;

docker run -v $(pwd)/collections:/etc/newman \
        --network=ascii-pay-server-newman-test-network \
        -it --rm asciipay/newman:latest \
        run ascii-pay-tests.postman_collection.json --env-var "base_url=http://server:3000"
