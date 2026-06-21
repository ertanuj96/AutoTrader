%%%-------------------------------------------------------------------
%%% @doc Top-level supervisor for all broker connections.
%%%
%%% Supervision strategy: one_for_one
%%% If a broker session crashes, only that session restarts.
%%% Other broker sessions continue unaffected.
%%%
%%% Child spec:
%%%   kill_switch    — GenServer: emergency order cancellation coordinator
%%%   rate_limiter   — GenServer: per-broker token bucket (SEBI < 10 OPS)
%%%   dhan_session   — GenServer: Dhan API session lifecycle
%%%   fyers_session  — GenServer: Fyers API session lifecycle
%%%   ind_session    — GenServer: INDstocks API session lifecycle
%%%-------------------------------------------------------------------
-module(broker_sup).
-behaviour(supervisor).

-export([start_link/0, init/1]).

start_link() ->
    supervisor:start_link({local, ?MODULE}, ?MODULE, []).

init([]) ->
    SupFlags = #{
        strategy => one_for_one,
        intensity => 5,       %% Max 5 restarts
        period => 60          %% within 60 seconds
    },

    KillSwitch = #{
        id => kill_switch,
        start => {kill_switch, start_link, []},
        restart => permanent,
        shutdown => 5000,
        type => worker,
        modules => [kill_switch]
    },

    RateLimiter = #{
        id => rate_limiter,
        start => {rate_limiter, start_link, []},
        restart => permanent,
        shutdown => 5000,
        type => worker,
        modules => [rate_limiter]
    },

    %% Broker sessions — each manages auth, reconnect, and order routing
    DhanSession = #{
        id => dhan_session,
        start => {broker_session, start_link, [dhan]},
        restart => permanent,
        shutdown => 10000,
        type => worker,
        modules => [broker_session]
    },

    FyersSession = #{
        id => fyers_session,
        start => {broker_session, start_link, [fyers]},
        restart => permanent,
        shutdown => 10000,
        type => worker,
        modules => [broker_session]
    },

    IndSession = #{
        id => ind_session,
        start => {broker_session, start_link, [indstocks]},
        restart => permanent,
        shutdown => 10000,
        type => worker,
        modules => [broker_session]
    },

    Children = [KillSwitch, RateLimiter, DhanSession, FyersSession, IndSession],
    {ok, {SupFlags, Children}}.
