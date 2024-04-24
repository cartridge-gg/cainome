config := --account katana-0 \
	--rpc http://0.0.0.0:5050

scarb_build := ./target/dev/contracts_
sierra := .contract_class.json
artifacts := ./abi/

generate_artifacts:
	scarb build
	mkdir -p ${artifacts}

	jq .abi ${scarb_build}simple_events${sierra} > ${artifacts}events.abi.json
	jq .abi ${scarb_build}simple_get_set${sierra} > ${artifacts}simple_get_set.abi.json
	jq .abi ${scarb_build}simple_types${sierra} > ${artifacts}simple_types.abi.json
	jq .abi ${scarb_build}components_contract${sierra} > ${artifacts}components.abi.json
	jq .abi ${scarb_build}enums${sierra} > ${artifacts}enums.abi.json
	jq .abi ${scarb_build}option_result${sierra} > ${artifacts}option_result.abi.json
	jq .abi ${scarb_build}simple_interface${sierra} > ${artifacts}simple_interface.abi.json
	jq .abi ${scarb_build}structs${sierra} > ${artifacts}structs.abi.json
	jq .abi ${scarb_build}byte_array${sierra} > ${artifacts}byte_array.abi.json


setup: setup_simple_get_set

# Declare and deploy the simple_get_set contract on katana.
setup_simple_get_set:
	@set -x; \
	scarb build; \
	class_hash=$$(starkli class-hash ${scarb_build}simple_get_set${sierra}); \
	starkli declare ${scarb_build}simple_get_set${sierra} ${config}; \
	sleep 2; \
	starkli deploy "$${class_hash}" --salt 0x1234 ${config}

setup_byte_array:
	@set -x; \
	scarb build; \
	class_hash=$$(starkli class-hash ${scarb_build}byte_array${sierra}); \
	starkli declare ${scarb_build}byte_array${sierra} ${config}; \
	sleep 2; \
	starkli deploy "$${class_hash}" --salt 0x1234 ${config}

# # Declare and deploy the basic contract on katana.
# setup_basic:
# 	$(eval class_hash=$(shell starkli class-hash target/dev/contracts_basic.sierra.json))
# 	starkli declare target/dev/contracts_basic.sierra.json ${config}
# 	starkli deploy ${class_hash} --salt 0x1234 ${config}

# # Declare and deploy the basic contract on katana.
# setup_gen:
# 	$(eval class_hash=$(shell starkli class-hash target/dev/contracts_gen.sierra.json))
# 	starkli declare target/dev/contracts_gen.sierra.json ${config}
# 	starkli deploy ${class_hash} --salt 0x1234 ${config}

# # Declare and deploy the event contract on katana.
# setup_event:
# 	$(eval class_hash=$(shell starkli class-hash target/dev/contracts_event.sierra.json))
# 	starkli declare target/dev/contracts_event.sierra.json ${config}
# 	starkli deploy ${class_hash} --salt 0x1234 ${config}
