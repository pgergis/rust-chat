port module Main exposing (..)

import Browser
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Json.Decode as D
import Dict
import Task
import Time

-- JavaScript usage: app.ports.websocketIn.send(response);
port websocketIn : (String -> msg) -> Sub msg
-- JavaScript usage: app.ports.websocketOut.subscribe(handler);
port websocketOut : String -> Cmd msg

port connectWs : String -> Cmd msg
port connectionResult : (Bool -> msg) -> Sub msg

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
    , otherUsers: List String
    , usernameSubmitAttempted : Bool
    , usernameSelected : Bool
    , time: Time.Posix
    , timeZone: Time.Zone
    }


init : () -> (Model, Cmd Msg)
init _ =
    ( Model [] "" "" [] False False (Time.millisToPosix 0) Time.utc
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
    | RecvServerResponse Bool
    | UpdateTime Time.Posix
    | AdjustTimeZone Time.Zone
    | NoOp

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
                , Cmd.batch [ submitChatMessage message
                            , Task.perform UpdateTime Time.now
                            ]
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
                updatedUsers = case D.decodeString (D.field "to_users" (D.dict D.string)) message of
                                 Err _ -> []
                                 Ok d -> Dict.values d
                fmtMessage =
                    ChatMessage
                        userId
                        userString
                        textString
                        model.time

                messages =
                    fmtMessage :: model.chatMessages
            in
                ( { model | chatMessages = messages, otherUsers = updatedUsers }
                , Task.perform UpdateTime Time.now
                )

        UpdateUsername username ->
            ( { model | username = username }
            , Cmd.none
            )

        UserRegister ->
            ( { model | usernameSubmitAttempted = True }
            , initRegisteredConnection model.username
            )

        GuestRegister ->
            ( { model | username = "You", usernameSelected = True }
            , initGuestConnection
            )

        RecvServerResponse wasSuccess ->
            ( { model | usernameSelected = wasSuccess, usernameSubmitAttempted = not wasSuccess }
            , Cmd.none
            )

        UpdateTime newTime -> ( { model | time = newTime }
                              , Cmd.none)

        AdjustTimeZone newZone -> ( { model | timeZone = newZone }
                                  , Cmd.none)

        NoOp -> (model, Cmd.none)


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
            , onKeyUp (keyUpSubmit UserRegister)
            , onInput UpdateUsername
            , class "u-full-width"
            , type_ "text"
            ]
            []
        , button
            [ onClick UserRegister
            , class "button-primary"
            , type_ "submit"
            ]
            [ text "Register" ]
        , span [ style "font-size" "80%"
               , style "color" "red"
               ]
               [ if not (List.isEmpty model.chatMessages) then
                     text " Looks like you were logged out; sign in again!"
                 else if model.usernameSubmitAttempted then
                          text " Username is blank or already taken!"
                 else text ""
               ]
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
            , onKeyUp (keyUpSubmit PostChatMessage)
            , onInput UpdateUserMessage
            , type_ "text"
            , style "margin-right" "0.5em"
            , align "left"
            ]
            []
        , button
            [ onClick PostChatMessage
            , type_ "submit"
            , class "button-primary"
            ]
            [ text "Submit" ]
        , div [] []
        , displayChatMessages model.username model.timeZone model.chatMessages
        , div [ style "color" "green"
              , style "padding-top" "5%"
              ]
              [ text "Connected users: " ]
        , displayConnectedUsers model.otherUsers
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

displayConnectedUsers : List String -> Html a
displayConnectedUsers users =
  div [ style "word-wrap" "normal" ]
      (List.map (\x -> div [] [ text x ]) users)

-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch [ websocketIn NewChatMessage
              , connectionResult RecvServerResponse
              ]


-- HELPERS


onKeyUp : (Int -> msg) -> Attribute msg
onKeyUp tagger = on "keyup" (D.map tagger keyCode)

keyUpSubmit : Msg -> Int -> Msg
keyUpSubmit action key = if key == 13 then action else NoOp

submitChatMessage : String -> Cmd Msg
submitChatMessage message =
    if message /= "" then websocketOut message else Cmd.none

printChatMessage : String ->  Time.Zone -> ChatMessage -> Html msg
printChatMessage myUsername myTimeZone msg =
    let
        col = if msg.fromHost then "blue" else "gray"
        timeString = (String.join ":" [ String.fromInt (Time.toHour myTimeZone msg.time)
                                      , String.fromInt (Time.toMinute myTimeZone msg.time)
                                      , String.fromInt (Time.toSecond myTimeZone msg.time)
                                      ])
    in
        div [align (if msg.username == myUsername then "right"
                    else if msg.fromHost then "center"
                    else "left")
            , style "word-wrap" "normal"
            ]
            [ span [style "color" col, style "font-size" "75%"] [text (msg.username)]
            , div [][]
            , div [ style "max-width" "40%"]
                  [ span [] [text msg.text]
                  , span [ style "color" "green"
                         , style "font-size" "80%"
                         ] [text (" " ++ timeString)]
                  ]
            ]


initGuestConnection : Cmd Msg
initGuestConnection = connectWs "/guest"

initRegisteredConnection : String -> Cmd Msg
initRegisteredConnection requestedUsername = connectWs ("/register?req_username=" ++ requestedUsername)
