#!/bin/env python3

import simusa
import json

class Controller(simusa.Controller):
    def __init__(self, config: dict, initial_time: float):
        self.last_time = initial_time
        if "speed" in config:
            self.speed: float = config["speed"]
        else:
            self.speed = 2

    def post_init(self, node: simusa.Node) -> None:
        pass

    def record(self) -> str:
        print("This is record from python!")
        return json.dumps({
            "last_time": self.last_time})

    def make_command(self, node: simusa.Node, input: simusa.ControllerInput, time: float) -> simusa.Command:
        self.last_time = time
        command = simusa.UnicycleCommand()
        command.left_wheel_speed = self.speed
        command.right_wheel_speed = self.speed
        return simusa.Command.from_unicycle_command(command)

    def pre_loop_hook(self, node: simusa.Node, time: float):
        pass

    def next_time_step(self):
        return None

class SimulatorAPI(simusa.PluginAPI):
    def get_controller(self, config: dict, global_config: dict, initial_time: float):
        config = json.loads(config)
        print(f"Config received by python: {type(config)} {config}")
        return Controller(config, initial_time)

def main():

    simulator_api = SimulatorAPI()

    simulator = simusa.Simulator.from_config(
        "config/config_controller.yaml", simulator_api
    )
    simulator.run()


if __name__ == "__main__":
    main()
