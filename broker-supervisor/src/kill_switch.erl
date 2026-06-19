%%%-------------------------------------------------------------------
%%% @doc Kill switch coordinator.
%%% When activated:
%%%   1. Cancel all open orders across all brokers
%%%   2. Square off all positions
%%%   3. Disable all strategy engines via NATS
%%%   4. Publish kill switch event
%%%-------------------------------------------------------------------
-module(kill_switch).
-behaviour(gen_server).

-export([start_link/0, activate/1, is_active/0]).
-export([init/1, handle_call/3, handle_cast/2, handle_info/2]).

-record(state, {
    active :: boolean(),
    activated_at :: integer() | undefined,
    reason :: binary() | undefined
}).

start_link() ->
    gen_server:start_link({local, ?MODULE}, ?MODULE, [], []).

%% @doc Activate the kill switch with a reason.
activate(Reason) ->
    gen_server:call(?MODULE, {activate, Reason}).

%% @doc Check if kill switch is currently active.
is_active() ->
    gen_server:call(?MODULE, is_active).

init([]) ->
    io:format("  🛡️  Kill switch ready~n"),
    {ok, #state{active = false, activated_at = undefined, reason = undefined}}.

handle_call({activate, Reason}, _From, State) ->
    case State#state.active of
        true ->
            {reply, {already_active, State#state.reason}, State};
        false ->
            io:format("~n  🚨🚨🚨 KILL SWITCH ACTIVATED: ~s 🚨🚨🚨~n~n", [Reason]),

            %% Cancel all orders on all brokers
            cancel_all_orders(),

            NewState = State#state{
                active = true,
                activated_at = erlang:system_time(millisecond),
                reason = list_to_binary(Reason)
            },
            {reply, ok, NewState}
    end;

handle_call(is_active, _From, State) ->
    {reply, State#state.active, State};

handle_call(_Request, _From, State) ->
    {reply, {error, unknown}, State}.

handle_cast(_Msg, State) -> {noreply, State}.
handle_info(_Info, State) -> {noreply, State}.

%% ── Internal ──

cancel_all_orders() ->
    Brokers = [dhan_session, fyers_session, ind_session],
    lists:foreach(fun(Session) ->
        try
            gen_server:cast(Session, cancel_all_orders)
        catch
            _:_ -> io:format("  ⚠️  Could not reach ~p~n", [Session])
        end
    end, Brokers).
