#!/bin/env python3

import simusa
import json
import time
from typing import List, Dict

class StateEstimator(simusa.StateEstimator):
    def __init__(self, config: dict, initial_time: float):
        self.last_time = initial_time

    def post_init(self, node: simusa.Node) -> None:
        node.make_channel("my_channel")
        self.client = node.subscribe(["/simusa/command/robot1"])

    def state(self) -> simusa.WorldState:
        return simusa.WorldState()

    def record(self) -> str:
        return "{}"

    def prediction_step(self, node: simusa.Node, command: simusa.Command, time: float):
        pass

    def correction_step(self, node: simusa.Node, observations: List[simusa.Observation], time: float):
        pass


    def next_time_step(self):
        return 10000000000
    
    def pre_loop_hook(self, node: simusa.Node, time: float):
        node.log_info("Master")
        if abs(5 - time) < 0.001:
            node.log_info("Try to send message")
            msg = simusa.MessageTypes.GoTo(simusa.GoToMessage([10,10]))
            self.client.send("/simusa/command/robot1", msg, time)
            node.log_info("Message sent")

        if abs(20 - time) < 0.001:
            node.log_info("Try to send message")
            msg = simusa.MessageTypes.GoTo(simusa.GoToMessage([-10,10]))
            self.client.send("/simusa/command/robot1", msg, time)
            node.log_info("Message sent")

        if abs(30 - time) < 0.001:
            node.log_info("Try to send message")
            msg = simusa.MessageTypes.GoTo(simusa.GoToMessage([-10,-10]))
            self.client.send("/simusa/command/robot1", msg, time)
            node.log_info("Message sent")


class SimulatorAPI(simusa.PluginAPI):
    def get_state_estimator(self, config, global_config, initial_time: float):
        config = json.loads(config)
        print(f"Config received by python: {type(config)} {config}")
        return StateEstimator(config, initial_time)

def main():

    simulator_api = SimulatorAPI()

    simulator = simusa.Simulator.from_config(
            "config/config_master.yaml", simulator_api
        )
    simulator.run()


if __name__ == "__main__":
    main()
