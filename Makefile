
run:
	@env $$(cat secrets/telegram.env | xargs) cargo run

build-image:
	@docker build -t mqtt-broker-alarm .

run-docker: build-image
	@docker run --env-file secrets/telegram.env --name mqtt-broker-alarm -d mqtt-broker-alarm --network host  

stop-docker:
	@docker stop mqtt-broker-alarm
