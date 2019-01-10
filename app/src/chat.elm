port module Main exposing (..)

import Browser
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Json.Decode as D
import Task
import Time

-- JavaScript usage: app.ports.websocketIn.send(response);
port websocketIn : (String -> msg) -> Sub msg
-- JavaScript usage: app.ports.websocketOut.subscribe(handler);
port websocketOut : String -> Cmd msg

port connectWs : String -> Cmd msg

main =
    Browser.element
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- MODEL

type alias ChatMessage =
    { fromHost: Bool
    , username: String
    , text: String
    , time: Time.Posix
    }


type alias Model =
    { chatMessages : List ChatMessage
    , userMessage : String
    , username : String
    , usernameSelected : Bool
    , time: Time.Posix
    , timeZone: Time.Zone
    }


init : () -> (Model, Cmd Msg)
init _ =
    ( Model [] "" "" False (Time.millisToPosix 0) Time.utc
    , Cmd.batch [ Task.perform UpdateTime Time.now
                , Task.perform AdjustTimeZone Time.here
                ]
    )



-- UPDATE


type Msg
    = PostChatMessage
    | UpdateUserMessage String
    | NewChatMessage String
    | UpdateUsername String
    | UserRegister
    | GuestRegister
    | UpdateTime Time.Posix
    | AdjustTimeZone Time.Zone

update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        PostChatMessage ->
            let
                message = model.userMessage
                username = model.username
                messages = (ChatMessage False username message model.time) :: model.chatMessages
            in
                ( { model | chatMessages = messages, userMessage = "" }
                , Cmd.batch [submitChatMessage message
                            , Task.perform UpdateTime Time.now]
                )

        UpdateUserMessage message ->
            ( { model | userMessage = message }
            , Cmd.none
            )

        NewChatMessage message ->
            let
                userId = case D.decodeString (D.field "id" D.int) message of
                             Err _ -> False
                             Ok i -> if i == 0 then True else False
                userString = case D.decodeString (D.field "user" D.string) message of
                                 Err _ -> "INVALID_USER"
                                 Ok u -> u
                textString = case D.decodeString (D.field "text" D.string) message of
                                    Err _ -> "INVALID_MESSAGE"
                                    Ok m -> m
                fmtMessage =
                    ChatMessage
                        userId
                        userString
                        textString
                        model.time

                messages =
                    fmtMessage :: model.chatMessages
            in
                ( { model | chatMessages = messages }
                , Task.perform UpdateTime Time.now
                )

        UpdateUsername username ->
            ( { model | username = username }
            , Cmd.none
            )

        UserRegister ->
            ( { model | usernameSelected = True }
            , initRegisteredConnection model.username
            )

        GuestRegister ->
            ( { model | username = "You", usernameSelected = True }
            , initGuestConnection
            )

        UpdateTime newTime -> ( { model | time = newTime }
                              , Cmd.none)

        AdjustTimeZone newZone -> ( { model | timeZone = newZone }
                                  , Cmd.none)



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "container" ]
        [ h3 [] [ text "Rusty Chat Room" ]
        , viewSelect model
        ]


viewSelect : Model -> Html Msg
viewSelect model =
    if model.usernameSelected then
        chatView model
    else
        enterNameView model


enterNameView : Model -> Html Msg
enterNameView model =
    div []
        [ label [] [ text "Enter your username for this chat: " ]
        , input
            [ autofocus True
            , value model.username
            , onInput UpdateUsername
            , class "u-full-width"
            , type_ "text"
            ]
            []
        , button
            [ onClick UserRegister
            , class "button-primary"
            ]
            [ text "Register" ]
        , div [] []
        , label [] [text "Or you can: "]
        , button
            [ onClick GuestRegister
            , class "button-primary"
            ]
            [ text "Connect as Guest" ]
        ]


chatView : Model -> Html Msg
chatView model =
    div []
        [ input
            [ placeholder "say something..."
            , autofocus True
            , value model.userMessage
            , onInput UpdateUserMessage
            , type_ "text"
            , style "margin-right" "0.5em"
            , align "left"
            ]
            []
        , button
            [ onClick PostChatMessage
            , class "button-primary"
            ]
            [ text "Submit" ]
        , div [] []
        , displayChatMessages model.username model.timeZone model.chatMessages
        ]


displayChatMessages : String -> Time.Zone -> List ChatMessage -> Html a
displayChatMessages myUsername myTimeZone chatMessages =
    div [align "center"
        , style "padding-top" "5%"
        , style "padding-left" "20%"
        , style "width" "55%"
        , style "display" "inline-block"
        , style "zoom" "1"
        , style "display*" "inline"]
        (List.map (printChatMessage myUsername myTimeZone) chatMessages)



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    websocketIn NewChatMessage



-- HELPERS


submitChatMessage : String -> Cmd Msg
submitChatMessage message =
    websocketOut message

printChatMessage : String ->  Time.Zone -> ChatMessage -> Html msg
printChatMessage myUsername myTimeZone msg =
    let
        col = if msg.fromHost then "red" else "blue"
        timeString = (String.join ":" [String.fromInt (Time.toHour myTimeZone msg.time)
                                      , String.fromInt (Time.toMinute myTimeZone msg.time)
                                      , String.fromInt (Time.toSecond myTimeZone msg.time)])
    in
        div [align (if msg.username == myUsername then "right"
                    else if msg.fromHost then "center"
                    else "left")
            , style "word-wrap" "normal"]
            [ span [style "color" col] [text (String.append "<" (String.append msg.username "> "))]
            , span [] [text msg.text]
            , span [style "color" "green", style "font-size" "80%"] [text (String.append " " timeString)]
            ]


initGuestConnection : Cmd Msg
initGuestConnection = connectWs "/guest"

initRegisteredConnection : String -> Cmd Msg
initRegisteredConnection requestedUsername = connectWs (String.append "/register?req_handle=" requestedUsername)
