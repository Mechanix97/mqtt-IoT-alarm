
run:
	@env $$(cat secrets/telegram.env | xargs) cargo run

build-image:
	@docker build -t mqtt-broker-alarm .

run-docker: build-image
	@docker run --network host --restart unless-stopped --env-file secrets/telegram.env \
		-v /home/lucas/home-assistant/data/cam0:/home/lucas/home-assistant/data/cam0:ro \
		-v /home/lucas/home-assistant/data/cam1:/home/lucas/home-assistant/data/cam1:ro \
		--name mqtt-broker-alarm -d mqtt-broker-alarm

stop-docker:
	@docker stop mqtt-broker-alarm
