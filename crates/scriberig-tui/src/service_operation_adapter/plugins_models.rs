//! Provides plugins models methods for the application operation adapter.

macro_rules! operation_plugins_models_methods {
    () => {
        /// Returns the runner's active upstream component registry.
        /// # Returns
        /// The active component mailbox when configured.
        fn extension_component_mailbox(&self) -> Option<rpi_extensions::ExtensionStatusMailbox> {
            self.service.extension_component_mailbox()
        }

        /// Returns plugin slash commands admitted by the local runtime runner.
        /// # Returns
        /// The active plugin command descriptors.
        fn plugin_commands(&self) -> Vec<rpi_extensions::isolated::WorkerCommandSpec> {
            self.agent_runner.plugin_commands()
        }

        /// Invokes a plugin slash command through the host-authorized runner.
        ///
        /// - `command`: Exact normalized plugin command identity.
        /// - `arguments`: Raw text following the command name.
        ///
        /// # Returns
        /// The plugin's structured command response.
        ///
        /// # Errors
        /// Returns a safe failure when the host runner rejects the command.
        fn invoke_plugin_command(
            &self,
            command: &str,
            arguments: &str,
        ) -> Result<serde_json::Value, String> {
            self.agent_runner.invoke_plugin_command(command, arguments)
        }

        /// Continues a plugin command with a structured action from its response UI.
        ///
        /// - `command`: Exact normalized command identity.
        /// - `arguments`: Raw arguments from the initial invocation.
        /// - `action`: Structured plugin UI submission.
        ///
        /// # Returns
        /// The next structured plugin UI response.
        ///
        /// # Errors
        /// Returns the injected agent runner's safe failure.
        fn invoke_plugin_command_action(
            &self,
            command: &str,
            arguments: &str,
            action: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            self.agent_runner
                .invoke_plugin_command_action(command, arguments, action)
        }

        /// Reads upstream property descriptors for one locally authorized model.
        ///
        /// - `context`: Local authorization context for model settings.
        /// - `provider_id`: Exact upstream provider identity.
        /// - `model_id`: Provider-native model identity.
        ///
        /// # Returns
        /// Upstream schema-backed descriptors for the selected model.
        ///
        /// # Errors
        /// Returns an authorization or configuration error.
        fn model_properties(
            &self,
            context: &ApplicationContext,
            provider_id: &str,
            model_id: &str,
        ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, ApplicationError> {
            require_local_model_config_edit(context)?;
            self.agent_runner.model_properties(provider_id, model_id)
        }

        /// Updates a provider property only through upstream schema validation.
        ///
        /// - `context`: Local authorization context for model settings.
        /// - `provider_id`: Exact upstream provider identity.
        /// - `model_id`: Provider-native model identity.
        /// - `key`: Upstream schema property key.
        /// - `value`: User value validated by upstream.
        ///
        /// # Returns
        /// Success after the upstream config accepts the value.
        ///
        /// # Errors
        /// Returns an authorization, validation, or persistence error.
        fn set_model_property(
            &self,
            context: &ApplicationContext,
            provider_id: &str,
            model_id: &str,
            key: &str,
            value: serde_json::Value,
        ) -> Result<(), ApplicationError> {
            require_local_model_config_edit(context)?;
            self.agent_runner
                .set_model_property(provider_id, model_id, key, value)
        }
    };
}
