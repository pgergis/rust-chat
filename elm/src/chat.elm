port module Main exposing (..)

import Browser
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import List

-- JavaScript usage: app.ports.websocketIn.send(response);
port websocketIn : (String -> msg) -> Sub msg
-- JavaScript usage: app.ports.websocketOut.subscribe(handler);
port websocketOut : String -> Cmd msg

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
    | SelectUsername


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
                , Cmd.batch [ submitChatMessage username message ]
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

        SelectUsername ->
            ( { model | usernameSelected = True }
            , Cmd.none
            )



-- VIEW


view : Model -> Html Msg
view model =
    div [ class "container" ]
        [ h3 [] [ text "Awesome Chat Room" ]
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
        [ label [] [ text "Enter your username for this chat" ]
        , input
            [ autofocus True
            , value model.username
            , onInput UpdateUsername
            , class "u-full-width"
            , type_ "text"
            ]
            []
        , button
            [ onClick SelectUsername
            , class "button-primary"
            ]
            [ text "Submit" ]
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


submitChatMessage : String -> String -> Cmd Msg
submitChatMessage username message =
    websocketOut (username ++ ": " ++ message)