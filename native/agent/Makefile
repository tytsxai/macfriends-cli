BUILD_DIR := build
CXX := clang++
COMMON_FLAGS := -std=c++17 -fobjc-arc -Wall -Wextra -O2
FRAMEWORKS := -framework Foundation
AGENT_SOURCES := src/agent.mm src/wechat_4_1_8_arm64.mm

.PHONY: all artifacts clean

all: artifacts

artifacts: $(BUILD_DIR)/libmacfriends_agent.dylib $(BUILD_DIR)/macfriends-host

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/libmacfriends_agent.dylib: $(AGENT_SOURCES) src/adapter.hpp | $(BUILD_DIR)
	$(CXX) $(COMMON_FLAGS) -dynamiclib $(AGENT_SOURCES) -o $@ $(FRAMEWORKS)

$(BUILD_DIR)/macfriends-host: src/host.mm | $(BUILD_DIR)
	$(CXX) $(COMMON_FLAGS) $< -o $@ $(FRAMEWORKS)

clean:
	rm -rf $(BUILD_DIR)
