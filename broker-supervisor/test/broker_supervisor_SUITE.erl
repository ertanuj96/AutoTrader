%%% @doc Common Test suite for the broker-supervisor OTP application.
%%%
%%% Covers supervisor lifecycle, kill switch gate semantics, and rate limiter
%%% SEBI compliance. Each test case starts and stops the relevant gen_servers
%%% in isolation to prevent state leaking between cases.

-module(broker_supervisor_SUITE).
-compile(export_all).
-include_lib("common_test/include/ct.hrl").

%% ── Suite callbacks ──

all() ->
    [
        supervisor_starts_and_links_children,
        kill_switch_initially_inactive,
        kill_switch_activates_once,
        kill_switch_idempotent_on_second_call,
        rate_limiter_allows_up_to_9_ops,
        rate_limiter_blocks_10th_op_per_second,
        rate_limiter_refills_after_one_second,
        rate_limiter_rejects_unknown_broker
    ].

%% ── Setup / Teardown ──

init_per_testcase(supervisor_starts_and_links_children, Config) ->
    Config;
init_per_testcase(_TestCase, Config) ->
    {ok, _KS} = kill_switch:start_link(),
    {ok, _RL} = rate_limiter:start_link(),
    Config.

end_per_testcase(supervisor_starts_and_links_children, _Config) ->
    ok;
end_per_testcase(_TestCase, _Config) ->
    (catch gen_server:stop(kill_switch)),
    (catch gen_server:stop(rate_limiter)),
    ok.

%% ── Test cases ──

supervisor_starts_and_links_children(_Config) ->
    {ok, Pid} = broker_sup:start_link(),
    ct:log("Supervisor PID: ~p", [Pid]),
    true = is_pid(Pid),
    %% Children should be registered
    timer:sleep(100),
    true = is_pid(whereis(kill_switch)),
    true = is_pid(whereis(rate_limiter)),
    gen_server:stop(Pid).

kill_switch_initially_inactive(_Config) ->
    false = kill_switch:is_active().

kill_switch_activates_once(_Config) ->
    ok = kill_switch:activate("CT: daily_loss_limit"),
    true = kill_switch:is_active().

kill_switch_idempotent_on_second_call(_Config) ->
    ok = kill_switch:activate("first"),
    {already_active, _Reason} = kill_switch:activate("second"),
    true = kill_switch:is_active().

rate_limiter_allows_up_to_9_ops(_Config) ->
    Results = [rate_limiter:try_acquire(dhan) || _ <- lists:seq(1, 9)],
    ct:log("Results: ~p", [Results]),
    OkCount = length([R || R <- Results, R =:= ok]),
    ct:log("OK count: ~p", [OkCount]),
    true = (OkCount =:= 9).

rate_limiter_blocks_10th_op_per_second(_Config) ->
    %% Drain the bucket
    [rate_limiter:try_acquire(fyers) || _ <- lists:seq(1, 9)],
    {error, throttled} = rate_limiter:try_acquire(fyers).

rate_limiter_refills_after_one_second(_Config) ->
    %% Exhaust the bucket
    [rate_limiter:try_acquire(indstocks) || _ <- lists:seq(1, 9)],
    {error, throttled} = rate_limiter:try_acquire(indstocks),
    %% Wait for 1-second refill window
    timer:sleep(1100),
    ok = rate_limiter:try_acquire(indstocks).

rate_limiter_rejects_unknown_broker(_Config) ->
    {error, unknown_broker} = rate_limiter:try_acquire(unknown_broker).
