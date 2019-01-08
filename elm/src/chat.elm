port module Main exposing (..)

import Browser
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)

-- JavaScript usage: app.ports.websocketIn.send(response);
port websocketIn : (String -> msg) -> Sub msg
-- JavaScript usage: app.ports.websocketOut.subscribe(handler);
port websocketOut : String -> Cmd msg

port connectWc : String -> Cmd msg

main =
    Browser.element
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- MODEL


type alias Model =
    { chatMessages : List String
    , userMessage : String
    , username : String
    , usernameSelected : Bool
    }


init : () -> (Model, Cmd Msg)
init _ =
    (Model [] "" "" False
    , Cmd.none
    )



-- UPDATE


type Msg
    = PostChatMessage
    | UpdateUserMessage String
    | NewChatMessage String
    | UpdateUsername String
    | UserRegister
    | GuestRegister


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        PostChatMessage ->
            let
                message =
                    model.userMessage

                username =
                    model.username

            in
                ( { model | userMessage = "" }
                , submitChatMessage message
                )

        UpdateUserMessage message ->
            ( { model | userMessage = message }
            , Cmd.none
            )

        NewChatMessage message ->
            let
                messages =
                    message :: model.chatMessages
            in
                ( { model | chatMessages = messages }
                , Cmd.none
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
            ( { model | usernameSelected = True }
            , initGuestConnection
            )



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
            ]
            []
        , button
            [ onClick PostChatMessage
            , class "button-primary"
            ]
            [ text "Submit" ]
        , displayChatMessages model.chatMessages
        ]


displayChatMessages : List String -> Html a
displayChatMessages chatMessages =
    div [] (List.map (\x -> div [] [ text x ]) chatMessages)



-- SUBSCRIPTIONS


subscriptions : Model -> Sub Msg
subscriptions model =
    websocketIn NewChatMessage



-- HELPERS


submitChatMessage : String -> Cmd Msg
submitChatMessage message =
    websocketOut message

initGuestConnection : Cmd Msg
initGuestConnection = connectWc "/guest"

initRegisteredConnection : String -> Cmd Msg
initRegisteredConnection requestedUsername = connectWc (String.append "/register?req_handle=" requestedUsername)
