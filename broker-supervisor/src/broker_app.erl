%%%-------------------------------------------------------------------
%%% @doc AutoTrader Broker Supervisor Application.
%%% Entry point for the OTP application. Starts the top-level supervisor.
%%%-------------------------------------------------------------------
-module(broker_app).
-behaviour(application).

-export([start/2, stop/1]).

start(_StartType, _StartArgs) ->
    io:format("~n🚀 AutoTrader Broker Supervisor v0.1.0~n"),
    broker_sup:start_link().

stop(_State) ->
    ok.
