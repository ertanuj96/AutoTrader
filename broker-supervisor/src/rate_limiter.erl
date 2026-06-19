%%%-------------------------------------------------------------------
%%% @doc Token bucket rate limiter — ensures SEBI OPS compliance.
%%% Each broker has a configurable max orders-per-second limit.
%%% Default: 9 OPS (under SEBI's 10 OPS HFT threshold).
%%%-------------------------------------------------------------------
-module(rate_limiter).
-behaviour(gen_server).

-export([start_link/0, try_acquire/1]).
-export([init/1, handle_call/3, handle_cast/2, handle_info/2]).

-record(state, {
    buckets :: map()  %% broker_atom => {tokens, last_refill_time, max_tokens}
}).

start_link() ->
    gen_server:start_link({local, ?MODULE}, ?MODULE, [], []).

%% @doc Try to acquire a token for the given broker. Returns ok | {error, throttled}.
try_acquire(Broker) ->
    gen_server:call(?MODULE, {try_acquire, Broker}).

init([]) ->
    %% Initialize buckets for each broker
    Buckets = #{
        dhan => {9, erlang:monotonic_time(millisecond), 9},
        fyers => {9, erlang:monotonic_time(millisecond), 9},
        indstocks => {9, erlang:monotonic_time(millisecond), 9}
    },
    {ok, #state{buckets = Buckets}}.

handle_call({try_acquire, Broker}, _From, State) ->
    Buckets = State#state.buckets,
    case maps:get(Broker, Buckets, undefined) of
        undefined ->
            {reply, {error, unknown_broker}, State};
        {Tokens, LastRefill, MaxTokens} ->
            Now = erlang:monotonic_time(millisecond),
            Elapsed = Now - LastRefill,

            %% Refill tokens: MaxTokens per second
            NewTokens = if
                Elapsed >= 1000 ->
                    min(MaxTokens, Tokens + MaxTokens);
                true ->
                    Tokens
            end,
            NewLastRefill = if Elapsed >= 1000 -> Now; true -> LastRefill end,

            case NewTokens > 0 of
                true ->
                    NewBuckets = maps:put(Broker,
                        {NewTokens - 1, NewLastRefill, MaxTokens}, Buckets),
                    {reply, ok, State#state{buckets = NewBuckets}};
                false ->
                    {reply, {error, throttled}, State}
            end
    end;

handle_call(_Request, _From, State) ->
    {reply, {error, unknown}, State}.

handle_cast(_Msg, State) -> {noreply, State}.
handle_info(_Info, State) -> {noreply, State}.

%% ── EUnit tests ──

-ifdef(TEST).
-include_lib("eunit/include/eunit.hrl").

setup() ->
    case whereis(?MODULE) of
        undefined -> {ok, _} = start_link();
        _Pid      -> ok
    end.

teardown(_) ->
    case whereis(?MODULE) of
        undefined -> ok;
        _         -> gen_server:stop(?MODULE)
    end.

rate_limiter_test_() ->
    {setup, fun setup/0, fun teardown/1, [
        {"known broker dhan acquires token", fun() ->
            ?assertEqual(ok, try_acquire(dhan))
        end},
        {"known broker fyers acquires token", fun() ->
            ?assertEqual(ok, try_acquire(fyers))
        end},
        {"unknown broker returns error", fun() ->
            ?assertMatch({error, unknown_broker}, try_acquire(unknown_exchange))
        end},
        {"dhan rate limit: 9 tokens then throttled", fun() ->
            %% Drain remaining tokens (we already consumed some above)
            drain_bucket(dhan, 20),
            %% Wait for refill
            timer:sleep(1100),
            %% Now consume exactly 9
            Results = [try_acquire(dhan) || _ <- lists:seq(1, 9)],
            OkCount = length([R || R <- Results, R =:= ok]),
            ?assertEqual(9, OkCount),
            %% 10th should be throttled
            ?assertMatch({error, throttled}, try_acquire(dhan))
        end},
        {"fyers rate limit enforced independently", fun() ->
            drain_bucket(fyers, 20),
            timer:sleep(1100),
            [ok = try_acquire(fyers) || _ <- lists:seq(1, 9)],
            ?assertMatch({error, throttled}, try_acquire(fyers))
        end}
    ]}.

drain_bucket(_Broker, 0) -> ok;
drain_bucket(Broker, N) ->
    try_acquire(Broker),
    drain_bucket(Broker, N - 1).

-endif.
