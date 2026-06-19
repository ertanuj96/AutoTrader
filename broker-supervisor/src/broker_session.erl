%%%-------------------------------------------------------------------
%%% @doc Broker session GenServer — manages a single broker connection.
%%%
%%% Handles:
%%%   - API authentication (2FA, token refresh)
%%%   - WebSocket connection lifecycle
%%%   - Daily session resets (brokers require re-auth each trading day)
%%%   - Order routing from NATS to broker API
%%%   - Automatic reconnection with exponential backoff
%%%
%%% On crash, the supervisor restarts this process automatically.
%%% The "let it crash" philosophy means we don't try to handle every
%%% edge case — we just restart cleanly.
%%%-------------------------------------------------------------------
-module(broker_session).
-behaviour(gen_server).

-export([start_link/1]).
-export([init/1, handle_call/3, handle_cast/2, handle_info/2, terminate/2]).

-record(state, {
    broker     :: atom(),         %% dhan | fyers | indstocks
    connected  :: boolean(),
    session_id :: binary(),
    config     :: map(),
    retry_count :: non_neg_integer(),
    max_retries :: non_neg_integer()
}).

%% ── API ──

start_link(Broker) ->
    Name = list_to_atom(atom_to_list(Broker) ++ "_session"),
    gen_server:start_link({local, Name}, ?MODULE, [Broker], []).

%% ── Callbacks ──

init([Broker]) ->
    io:format("  ✅ ~p session starting~n", [Broker]),
    %% Load broker config from application env
    {ok, BrokerConfigs} = application:get_env(broker_supervisor, brokers),
    Config = proplists:get_value(Broker, BrokerConfigs, #{}),

    State = #state{
        broker = Broker,
        connected = false,
        session_id = <<>>,
        config = Config,
        retry_count = 0,
        max_retries = 10
    },

    %% Schedule initial connection attempt
    self() ! connect,
    {ok, State}.

handle_call(status, _From, State) ->
    {reply, #{
        broker => State#state.broker,
        connected => State#state.connected,
        retry_count => State#state.retry_count
    }, State};

handle_call(_Request, _From, State) ->
    {reply, {error, unknown_request}, State}.

handle_cast({place_order, OrderIntent}, State) ->
    case State#state.connected of
        true ->
            io:format("  ⚡ ~p: placing order ~p~n", [State#state.broker, OrderIntent]),
            %% TODO: Call broker-specific REST API
            {noreply, State};
        false ->
            io:format("  ⚠️  ~p: not connected, dropping order~n", [State#state.broker]),
            {noreply, State}
    end;

handle_cast(_Msg, State) ->
    {noreply, State}.

handle_info(connect, State) ->
    %% TODO: Implement broker-specific authentication
    %% For now, simulate connection
    io:format("  🔌 ~p: connecting (attempt ~p)~n",
              [State#state.broker, State#state.retry_count + 1]),

    %% Simulate: connection succeeds
    NewState = State#state{
        connected = true,
        retry_count = 0,
        session_id = <<"simulated_session">>
    },
    io:format("  ✅ ~p: connected~n", [State#state.broker]),

    %% Schedule daily re-auth (brokers reset sessions at ~5:30 AM IST)
    schedule_daily_reset(),
    {noreply, NewState};

handle_info(daily_reset, State) ->
    io:format("  🔄 ~p: daily session reset~n", [State#state.broker]),
    NewState = State#state{connected = false, session_id = <<>>},
    self() ! connect,
    {noreply, NewState};

handle_info(reconnect, State) ->
    case State#state.retry_count >= State#state.max_retries of
        true ->
            io:format("  ❌ ~p: max retries exceeded, crashing for supervisor restart~n",
                      [State#state.broker]),
            {stop, max_retries_exceeded, State};
        false ->
            %% Exponential backoff: 1s, 2s, 4s, 8s, ...
            Delay = min(30000, 1000 * (1 bsl State#state.retry_count)),
            io:format("  🔄 ~p: reconnecting in ~p ms~n", [State#state.broker, Delay]),
            erlang:send_after(Delay, self(), connect),
            {noreply, State#state{retry_count = State#state.retry_count + 1}}
    end;

handle_info(_Info, State) ->
    {noreply, State}.

terminate(Reason, State) ->
    io:format("  💀 ~p session terminated: ~p~n", [State#state.broker, Reason]),
    ok.

%% ── Internal ──

schedule_daily_reset() ->
    %% Reset at 5:30 AM IST next day (simplified: reset in 24h)
    erlang:send_after(24 * 60 * 60 * 1000, self(), daily_reset).
